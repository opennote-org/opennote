//! Integration tests for the delegating Streamable HTTP transport.

mod common;

use actix_web::{App, HttpServer, web};
use futures::StreamExt;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

use common::calculator::Calculator;
use common::sse::find_in_sse_stream;

/// Starts a server on an ephemeral port and returns its base MCP URL.
async fn start_server(stateful_mode: bool) -> String {
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(stateful_mode)
        .build();

    let server = HttpServer::new(move || {
        App::new().service(web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("bind");

    let addr = *server.addrs().first().expect("addr");
    let handle = server.run();
    tokio::spawn(async move {
        let _ = handle.await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    format!("http://{addr}/mcp")
}

/// Reads an SSE or JSON response body and returns the first JSON-RPC payload it carries.
async fn first_json_rpc_payload(response: reqwest::Response) -> Value {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if !content_type.contains("text/event-stream") {
        return response.json().await.expect("json body");
    }
    find_in_sse_stream(response, |payload| Some(payload.clone()))
        .await
        .expect("an SSE data event carrying a JSON-RPC payload")
}

#[actix_web::test]
async fn call_tool_result_carries_result_type_for_2026_07_28_peer() {
    let url = start_server(false).await;
    let client = reqwest::Client::new();

    let initialize = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("initialize");
    let initialize = first_json_rpc_payload(initialize).await;
    assert_eq!(
        initialize.pointer("/result/protocolVersion"),
        Some(&json!("2026-07-28")),
        "server must negotiate 2026-07-28"
    );

    let call = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        // SEP-2243 requires the request method and target name to be mirrored into
        // headers, and the server rejects a mismatch with JSON-RPC -32020.
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "sum")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "sum",
                "arguments": {"a": 1, "b": 2},
                // A 2026-07-28 peer carries its protocol signals per request, not
                // only in the handshake. Both fields are required; omitting either
                // is answered with JSON-RPC -32602.
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {}
                }
            }
        }))
        .send()
        .await
        .expect("tools/call");
    let call = first_json_rpc_payload(call).await;

    assert_eq!(
        call.pointer("/result/resultType"),
        Some(&json!("complete")),
        "SEP-2322 resultType must survive to the wire for a 2026-07-28 peer"
    );
}

#[actix_web::test]
async fn legacy_peer_still_receives_a_session_id() {
    let url = start_server(true).await;
    let response = reqwest::Client::new()
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("initialize");

    assert!(
        response.headers().contains_key("mcp-session-id"),
        "a 2025-03-26 peer against a session-based deployment must receive a session id"
    );
    let body = first_json_rpc_payload(response).await;
    assert_eq!(
        body.pointer("/result/protocolVersion"),
        Some(&json!("2025-03-26"))
    );
}

#[actix_web::test]
async fn notifications_are_accepted_on_the_sessionless_path() {
    let url = start_server(false).await;
    let response = reqwest::Client::new()
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .send()
        .await
        .expect("send");
    assert_eq!(
        response.status(),
        202,
        "a notification carries no id, so it is acknowledged rather than answered"
    );
}

#[actix_web::test]
async fn sse_frames_arrive_before_the_stream_completes() {
    let url = start_server(true).await;
    let client = reqwest::Client::new();

    let init = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("initialize");
    let session_id = init
        .headers()
        .get("mcp-session-id")
        .expect("session id")
        .to_str()
        .expect("utf-8")
        .to_string();

    // The standalone GET stream stays open indefinitely, so any byte received from it
    // proves the bridge does not wait for completion before forwarding. A bridge that
    // collected the body would never reach the point of writing response headers, which
    // is why even the handshake below is bounded.
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        client
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", &session_id)
            .send(),
    )
    .await
    .expect("bridge buffered the SSE stream instead of answering with headers")
    .expect("open sse stream");
    assert_eq!(response.status(), 200);

    let mut stream = response.bytes_stream();
    let first = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("bridge buffered the SSE stream instead of forwarding frames")
        .expect("stream ended")
        .expect("chunk");
    assert!(
        !first.is_empty(),
        "the first forwarded chunk must carry the priming event, not an empty frame"
    );

    // The stream must still be open, otherwise the byte above could have been
    // forwarded by a bridge that buffers until completion.
    let second = tokio::time::timeout(Duration::from_millis(500), stream.next()).await;
    assert!(
        second.is_err(),
        "the standalone stream must still be open after its first frame, got: {second:?}"
    );
    // Dropping the stream here closes the connection.
}

#[actix_web::test]
async fn rejects_a_host_header_outside_the_loopback_default() {
    let url = start_server(false).await;
    let response = reqwest::Client::new()
        .post(&url)
        .header("Host", "evil.example.com")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}))
        .send()
        .await
        .expect("send");
    assert_eq!(
        response.status(),
        403,
        "loopback default must reject a foreign Host"
    );
}

/// The DNS-rebinding defence needs an authority to check. An actix request URI
/// carries none, so an absent `Host` header is rejected outright rather than
/// treated as loopback. HTTP/1.1 clients always send `Host`; in-process actix
/// test harnesses do not unless told to.
#[actix_web::test]
async fn rejects_a_request_without_a_host_header() {
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .build();

    let app = actix_web::test::init_service(
        App::new().service(web::scope("/mcp").service(service.scope())),
    )
    .await;

    let request = actix_web::test::TestRequest::post()
        .uri("/mcp/")
        .insert_header(("content-type", "application/json"))
        .insert_header(("accept", "application/json, text/event-stream"))
        .set_json(json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}))
        .to_request();

    let response = actix_web::test::call_service(&app, request).await;

    assert_eq!(
        response.status(),
        400,
        "an absent Host header must be rejected, not defaulted to loopback"
    );
    let body = actix_web::test::read_body(response).await;
    assert_eq!(body, "Bad Request: missing Host header");
}

#[actix_web::test]
async fn accepts_a_host_named_in_allowed_hosts() {
    let service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(false)
        .allowed_hosts(vec!["deploy.example.com".to_string()])
        .build();

    let server = HttpServer::new(move || {
        App::new().service(web::scope("/mcp").service(service.clone().scope()))
    })
    .bind("127.0.0.1:0")
    .expect("bind");

    let addr = *server.addrs().first().expect("addr");
    let handle = server.run();
    let server_task = tokio::spawn(async move {
        let _ = handle.await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Host", "deploy.example.com")
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("send");

    assert_ne!(
        response.status(),
        403,
        "a Host named in allowed_hosts must not be rejected"
    );
    assert!(response.status().is_success());

    server_task.abort();
}
