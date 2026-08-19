//! Integration tests for `Mcp-Session-Id` handling.
//!
//! The transport delegates every session decision to rmcp, so rmcp's responses
//! are the contract these tests pin. `POST` and `GET` carrying an unrecognized
//! or empty session id answer `404 Not Found`, letting the client recover by
//! starting a new session via an `InitializeRequest` without a session id.
//! `DELETE` is idempotent and answers `202 Accepted` whether or not the session
//! exists. A missing session id on `GET` or `DELETE` answers `400 Bad Request`,
//! and a `POST` that is neither an `initialize` request nor bound to a session
//! answers `422 Unprocessable Entity`. In stateless mode the header is ignored.

mod common;

use actix_web::{App, HttpServer};
use common::calculator::Calculator;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const MISSING_SESSION_ID_BODY: &str = "Bad Request: Session ID is required";
const SESSION_NOT_FOUND_BODY: &str = "Not Found: Session not found";
const NOT_AN_INITIALIZE_REQUEST_BODY: &str = "Unexpected message, expect initialize request";

struct TestServer {
    url: String,
    client: reqwest::Client,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn spawn(stateful: bool) -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("rmcp_actix_web=debug")
            .with_test_writer()
            .try_init();

        let service = StreamableHttpService::builder()
            .service_factory(Arc::new(|| Ok(Calculator::new())))
            .session_manager(Arc::new(LocalSessionManager::default()))
            .stateful_mode(stateful)
            .build();

        let server = HttpServer::new(move || {
            App::new().service(actix_web::web::scope("/").service(service.clone().scope()))
        })
        .bind("127.0.0.1:0")
        .expect("Failed to bind server");

        let addr = *server.addrs().first().unwrap();
        let server_handle = server.run();
        let task = tokio::spawn(async move {
            let _ = server_handle.await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        Self {
            url: format!("http://{addr}"),
            client: reqwest::Client::new(),
            task,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[actix_web::test]
async fn post_with_unknown_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

#[actix_web::test]
async fn get_with_unknown_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .get(&server.url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

/// `DELETE` is idempotent: deleting a session that does not exist reaches the
/// same end state as deleting one that does, so it is acknowledged rather than
/// rejected.
#[actix_web::test]
async fn delete_with_unknown_session_id_returns_202() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .delete(&server.url)
        .header("Mcp-Session-Id", "definitely-not-a-real-session")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let body = response.text().await.expect("Failed to read response body");
    assert!(body.is_empty(), "expected an empty body, got: {body:?}");
}

#[actix_web::test]
async fn get_with_missing_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .get(&server.url)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

#[actix_web::test]
async fn delete_with_missing_session_id_returns_400() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .delete(&server.url)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, MISSING_SESSION_ID_BODY);
}

/// Without a session id the only message a stateful server can place is an
/// `initialize` request, so anything else is well-formed but unprocessable.
#[actix_web::test]
async fn post_without_session_id_and_non_initialize_returns_422() {
    let server = TestServer::spawn(true).await;

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, NOT_AN_INITIALIZE_REQUEST_BODY);
}

/// An empty header value is not a missing header: it is a session id that no
/// session matches.
#[actix_web::test]
async fn post_with_empty_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "")
        .json(&tools_list_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

/// An empty header value is not a missing header: it is a session id that no
/// session matches.
#[actix_web::test]
async fn get_with_empty_session_id_returns_404() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .get(&server.url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", "")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

/// `DELETE` is idempotent, so an empty session id is acknowledged for the same
/// reason an unknown one is.
#[actix_web::test]
async fn delete_with_empty_session_id_returns_202() {
    let server = TestServer::spawn(true).await;

    let response = server
        .client
        .delete(&server.url)
        .header("Mcp-Session-Id", "")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let body = response.text().await.expect("Failed to read response body");
    assert!(body.is_empty(), "expected an empty body, got: {body:?}");
}

/// The negative `DELETE` tests answer `202` for any session id, so on their own
/// they would also pass against a transport that ignored `DELETE` entirely.
/// This pins that a `DELETE` naming a live session actually ends it.
#[actix_web::test]
async fn delete_with_a_live_session_id_ends_the_session() {
    let server = TestServer::spawn(true).await;

    let initialize_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "delete-live-session-test",
                "version": "0.0.0"
            }
        },
        "id": 1
    });

    let initialized = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&initialize_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(initialized.status(), reqwest::StatusCode::OK);
    let session_id = initialized
        .headers()
        .get("Mcp-Session-Id")
        .expect("a stateful initialize must answer with a session id")
        .to_str()
        .expect("session id must be UTF-8")
        .to_string();
    let _ = initialized.text().await;

    let deleted = server
        .client
        .delete(&server.url)
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(deleted.status(), reqwest::StatusCode::ACCEPTED);

    let reused = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", &session_id)
        .json(&json!({"jsonrpc": "2.0", "method": "tools/list", "id": 2}))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        reused.status(),
        reqwest::StatusCode::NOT_FOUND,
        "a deleted session id must no longer resolve"
    );
    let body = reused.text().await.expect("Failed to read response body");
    assert_eq!(body, SESSION_NOT_FOUND_BODY);
}

#[actix_web::test]
async fn stateless_post_with_session_id_header_is_ignored() {
    let server = TestServer::spawn(false).await;

    let initialize_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "stateless-session-id-test",
                "version": "0.0.0"
            }
        },
        "id": 1
    });

    let response = server
        .client
        .post(&server.url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Session-Id", "stale-from-previous-deployment")
        .json(&initialize_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await.expect("Failed to read response body");
    let data = body
        .strip_prefix("data: ")
        .and_then(|rest| rest.split("\n\n").next())
        .expect("body must be a `data:` SSE frame");
    let payload: serde_json::Value =
        serde_json::from_str(data).expect("`data:` payload must be JSON");
    assert_eq!(
        payload["jsonrpc"], "2.0",
        "expected JSON-RPC frame: {payload:?}"
    );
    assert_eq!(payload["id"], 1, "id must echo the initialize request");
    assert!(
        payload["result"]["protocolVersion"].is_string(),
        "expected initialize result with protocolVersion, got: {payload:?}"
    );
}
