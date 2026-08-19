//! Tests for framework-level scope composition
//!
//! These tests verify that StreamableHttp services can be
//! mounted at custom paths using actix-web's scope composition.

use std::sync::Arc;

use actix_web::{App, test, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;

mod common;
use common::calculator::Calculator;
use common::sse::find_in_sse_body;

/// Builds a stateful service for mounting under a scope.
fn service() -> StreamableHttpService<Calculator> {
    StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build()
}

/// Builds an `initialize` request addressed to `uri`.
fn initialize_request(uri: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri(uri)
        // The in-process test client sends no Host header, which the transport's
        // DNS-rebinding defence rejects outright.
        .insert_header(("host", "localhost"))
        .insert_header(("content-type", "application/json"))
        .insert_header(("accept", "application/json, text/event-stream"))
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
}

/// Asserts the response is a completed `initialize` handshake, not merely a
/// route that resolved to some status.
async fn assert_initialized(response: actix_web::dev::ServiceResponse) {
    assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    assert!(
        response.headers().contains_key("mcp-session-id"),
        "a stateful initialize must answer with a session id"
    );

    let body = test::read_body(response).await;
    let body = String::from_utf8(body.to_vec()).expect("utf-8 body");
    let payload = find_in_sse_body(&body, |value| {
        value
            .pointer("/result/protocolVersion")
            .is_some()
            .then(|| value.clone())
    })
    .unwrap_or_else(|| panic!("expected an initialize result, got: {body:?}"));

    assert_eq!(payload["id"], 1, "id must echo the initialize request");
}

/// The service is mounted under a single scope, so it sees one level of actix
/// path stripping.
#[actix_web::test]
async fn test_streamable_http_service_scope_composition() {
    let app = test::init_service(
        App::new().service(web::scope("/api/v2/mcp").service(service().scope())),
    )
    .await;

    let response = test::call_service(&app, initialize_request("/api/v2/mcp/").to_request()).await;

    assert_initialized(response).await;
}

/// The service is mounted under scopes nested inside one another, so each
/// enclosing scope strips its own prefix before the transport sees the request.
/// A path-stripping bug that only appears at the second level cannot hide here.
#[actix_web::test]
async fn test_streamable_http_service_nested_scope_composition() {
    let app = test::init_service(
        App::new().service(
            web::scope("/api")
                .service(web::scope("/v2").service(web::scope("/mcp").service(service().scope()))),
        ),
    )
    .await;

    let response = test::call_service(&app, initialize_request("/api/v2/mcp/").to_request()).await;

    assert_initialized(response).await;
}

/// The service is mounted at the application root, so actix strips no prefix at
/// all and the transport's own scope carries an empty one. This is the shape the
/// crate documentation and the examples use.
#[actix_web::test]
async fn test_streamable_http_service_root_mount_composition() {
    let app = test::init_service(App::new().service(service().scope())).await;

    let response = test::call_service(&app, initialize_request("/").to_request()).await;

    assert_initialized(response).await;
}
