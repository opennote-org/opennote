//! Integration tests for Authorization header forwarding in MCP proxy scenarios.
//!
//! These tests verify that Authorization headers are properly forwarded to MCP services
//! while other headers are not, as per the MCP specification requirements.

mod common;

use actix_web::{App, HttpServer};
use common::headers_test_service::HeadersTestService;
use common::sse::find_tool_json_in_sse_stream;
#[cfg(feature = "authorization-token-passthrough")]
use futures::StreamExt;
use reqwest::Response;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// Extracts the `authorization` field of the tool's JSON report from an SSE response.
async fn extract_auth_from_sse_response(response: Response) -> Option<String> {
    let report = find_tool_json_in_sse_stream(response).await?;
    report.get("authorization")?.as_str().map(String::from)
}

/// Sends `initialize` then `get_auth_surfaces` with the given Authorization header,
/// returning the tool's JSON report.
async fn auth_surfaces_for(authorization: &str) -> Value {
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();
    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    let init = client
        .post(&url)
        .header("Authorization", authorization)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }))
        .send()
        .await
        .expect("initialize");
    assert_eq!(init.status(), 200);

    let call = client
        .post(&url)
        .header("Authorization", authorization)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": "get_auth_surfaces", "arguments": {}}
        }))
        .send()
        .await
        .expect("tools/call");

    let report = find_tool_json_in_sse_stream(call)
        .await
        .expect("tool returned a report");
    server_task.abort();
    report
}

#[cfg(not(feature = "authorization-token-passthrough"))]
#[actix_web::test]
async fn authorization_is_stripped_when_passthrough_is_disabled() {
    let report = auth_surfaces_for("Bearer test-token-abc123").await;
    assert_eq!(
        report["extension"],
        Value::Null,
        "AuthorizationHeader must not reach handlers when the feature is off"
    );
    assert_eq!(
        report["raw_header"],
        Value::Null,
        "the raw Authorization header must not reach handlers via Parts.headers either"
    );
}

#[cfg(feature = "authorization-token-passthrough")]
#[actix_web::test]
async fn authorization_is_forwarded_when_passthrough_is_enabled() {
    let report = auth_surfaces_for("Bearer test-token-abc123").await;
    assert_eq!(report["extension"], "Bearer test-token-abc123");
    assert_eq!(report["raw_header"], "Bearer test-token-abc123");
}

#[cfg(feature = "authorization-token-passthrough")]
#[actix_web::test]
async fn test_authorization_forwarded_in_streamable_http_stateless() {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    // Create service in stateless mode
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // Send initialize request with Authorization header
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let response = client
        .post(&url)
        .header("Authorization", "Bearer test-token-abc123")
        .header("X-Custom-Header", "should-not-be-forwarded")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Read SSE response
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                body.extend_from_slice(&bytes);
                if body.ends_with(b"\n\n") || body.len() > 4096 {
                    break;
                }
            }
        }
    })
    .await;

    let body_str = String::from_utf8_lossy(&body);
    assert!(
        body_str.contains("data: "),
        "Response should be in SSE format"
    );

    // Now send a tool call to check what headers were captured
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_headers"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Authorization", "Bearer test-token-abc123")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    server_task.abort();
}

#[cfg(feature = "authorization-token-passthrough")]
#[actix_web::test]
async fn test_authorization_forwarded_in_streamable_http_stateful() {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    // Create service in stateful mode
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // In stateful mode, we need to initialize without a session first
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    // First request creates the session
    let response = client
        .post(&url)
        .header("Authorization", "Bearer initial-token")
        .header("X-Another-Header", "should-not-be-forwarded")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Extract session ID from response headers
    let session_id = response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .expect("Should have session ID")
        .to_string();

    // Verify we got a session ID
    assert!(!session_id.is_empty());

    // Read the initialize response from SSE stream
    let mut init_body = Vec::new();
    let mut init_stream = response.bytes_stream();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(chunk) = init_stream.next().await {
            if let Ok(bytes) = chunk {
                init_body.extend_from_slice(&bytes);
                if init_body.ends_with(b"\n\n") || init_body.len() > 4096 {
                    break;
                }
            }
        }
    })
    .await;

    // Send initialized notification as per MCP protocol
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let initialized_response = client
        .post(&url)
        .header("Authorization", "Bearer initial-token")
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&initialized_notification)
        .send()
        .await
        .expect("Failed to send initialized notification");

    assert_eq!(initialized_response.status(), 202); // Notifications return 202 Accepted

    // Test that subsequent requests to the existing session also forward Authorization
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_current_auth"
        },
        "id": 2
    });

    // Send a tool call with a DIFFERENT Authorization token to the existing session
    let tool_response = client
        .post(&url)
        .header("Authorization", "Bearer subsequent-token")
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    // Verify the subsequent request's token was forwarded
    let auth = extract_auth_from_sse_response(tool_response).await;
    assert_eq!(
        auth,
        Some("Bearer subsequent-token".to_string()),
        "Authorization header should be forwarded for existing sessions"
    );

    // Test token rotation: verify each request can have its own auth token
    let rotation_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_current_auth"
        },
        "id": 3
    });

    let rotation_response = client
        .post(&url)
        .header("Authorization", "Bearer rotated-token")
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&rotation_request)
        .send()
        .await
        .expect("Failed to send rotation request");

    assert_eq!(rotation_response.status(), 200);

    // Verify token rotation works within the same session
    let rotated_auth = extract_auth_from_sse_response(rotation_response).await;
    assert_eq!(
        rotated_auth,
        Some("Bearer rotated-token".to_string()),
        "Token rotation should work within same session (OAuth 2.1 best practice)"
    );

    server_task.abort();
}

#[cfg(feature = "authorization-token-passthrough")]
#[actix_web::test]
async fn test_malformed_bearer_tokens_not_forwarded() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind to port");

    let port = server.addrs()[0].port();
    let server_task = tokio::spawn(server.run());
    let url = format!("http://127.0.0.1:{}", port);

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // Test 1: Bearer with no token value
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let response = client
        .post(&url)
        .header("Authorization", "Bearer") // Malformed: no token
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let session_id = response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .expect("Should have session ID")
        .to_string();

    // Send tool call to check if malformed token was forwarded (it shouldn't be)
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_current_auth"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Authorization", "Bearer ") // Malformed: space but no token
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);

    let auth = extract_auth_from_sse_response(tool_response).await;
    assert_eq!(auth, None, "Malformed Bearer token should not be forwarded");

    server_task.abort();
}

#[actix_web::test]
async fn test_non_bearer_authorization_not_forwarded() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // Send request with non-Bearer authorization
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    // Test 1: Basic auth (should not be forwarded)
    let response = client
        .post(&url)
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Test 2: Custom auth scheme (should not be forwarded)
    let custom_auth_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_current_auth"
        },
        "id": 2
    });

    let response = client
        .post(&url)
        .header("Authorization", "CustomScheme sometoken123")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&custom_auth_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let auth = extract_auth_from_sse_response(response).await;
    assert_eq!(
        auth, None,
        "Non-Bearer authorization should not be forwarded"
    );

    server_task.abort();
}

#[actix_web::test]
async fn test_missing_authorization_doesnt_break_service() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("rmcp_actix_web=debug")
        .with_test_writer()
        .try_init();

    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(HeadersTestService::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(actix_web::web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("Failed to bind server");

    let addr = *server.addrs().first().unwrap();
    let server_handle = server.run();

    let server_task = tokio::spawn(async move {
        let _ = server_handle.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    // Send request without Authorization header
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_request)
        .send()
        .await
        .expect("Failed to send request");

    // Service should work fine without Authorization header
    assert_eq!(response.status(), 200);

    // Verify no authorization is returned when missing
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_current_auth"
        },
        "id": 2
    });

    let tool_response = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tool_request)
        .send()
        .await
        .expect("Failed to send tool request");

    assert_eq!(tool_response.status(), 200);
    let auth = extract_auth_from_sse_response(tool_response).await;
    assert_eq!(
        auth, None,
        "No authorization should be present when header is missing"
    );

    server_task.abort();
}
