//! Runtime-observable coverage for the transport's rmcp config knobs.

use std::sync::Arc;

use actix_web::{App, test, web};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;

mod common;
use common::calculator::Calculator;

/// Builds a service whose accepted body size is capped at `limit` bytes.
fn service_with_body_limit(limit: usize) -> StreamableHttpService<Calculator> {
    StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(Calculator::new())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .max_request_body_bytes(limit)
        .build()
}

/// Builds a well-formed `initialize` request body.
fn initialize_body() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Builds a `POST` addressed at the mounted transport.
///
/// The in-process test client sends no `Host` header, which the transport's
/// DNS-rebinding defence rejects outright, so one is supplied here.
fn post_request() -> test::TestRequest {
    test::TestRequest::post()
        .uri("/mcp")
        .insert_header(("host", "localhost"))
        .insert_header(("content-type", "application/json"))
        .insert_header(("accept", "application/json, text/event-stream"))
}

#[actix_web::test]
async fn oversized_request_body_is_rejected_with_413() {
    let service = service_with_body_limit(256);
    let app =
        test::init_service(App::new().service(web::scope("/mcp").service(service.scope()))).await;

    let mut oversized = initialize_body();
    oversized["params"]["clientInfo"]["version"] = serde_json::Value::String("x".repeat(4096));
    let request = post_request().set_json(&oversized).to_request();

    let response = test::call_service(&app, request).await;

    assert_eq!(response.status().as_u16(), 413);
}

// Pins the re-export contract: every public name `rmcp_actix_web::transport` offers is
// still reachable through that path, under that name, with that shape. Nothing here
// runs — the check is that dropping or demoting a public item stops this file compiling,
// which is what keeps a re-export from vanishing silently.
const _: fn() = || {
    use rmcp_actix_web::transport::{
        AuthorizationHeader, CancellationToken, Extensions, LocalSessionManager, OnRequestHook,
        SessionManager, SessionStore, StreamableHttpService, StreamableHttpServiceBuilder, http,
        on_request_extensions, streamable_http_server,
    };

    fn takes_a_session_manager<M: SessionManager>(_manager: Arc<M>) {}

    let _: Option<&http::request::Parts> = None;
    let _: Option<Arc<dyn SessionStore>> = None;
    let _: Option<CancellationToken> = None;
    let _: Option<Extensions> = None;
    let _: Option<Arc<OnRequestHook>> = None;
    let _: Option<AuthorizationHeader> = None;
    let _: fn(&Extensions) -> Option<&Extensions> = on_request_extensions;
    let _: Option<streamable_http_server::StreamableHttpService<Calculator>> = None;
    let _: StreamableHttpServiceBuilder<Calculator> = StreamableHttpService::builder();
    takes_a_session_manager(Arc::new(LocalSessionManager::default()));
};

// The `legacy-transport` module is a public path only when its feature is on.
#[cfg(feature = "legacy-transport")]
const _: fn() = || {
    use rmcp_actix_web::transport::legacy_streamable_http_server;

    let _: Option<legacy_streamable_http_server::StreamableHttpService<Calculator>> = None;
};

/// The configured limit, not something unconditional, is what decides the `413`.
///
/// One body, two services: below the limit it is served, above it is rejected. A limit
/// that stopped reaching rmcp would leave both under rmcp's own default and collapse the
/// two outcomes into one.
#[actix_web::test]
async fn the_configured_limit_decides_whether_a_body_is_rejected_with_413() {
    let body = initialize_body();
    let body_len = serde_json::to_vec(&body)
        .expect("the body serializes")
        .len();

    let under = service_with_body_limit(body_len * 2);
    let over = service_with_body_limit(body_len / 2);

    let app_under =
        test::init_service(App::new().service(web::scope("/mcp").service(under.scope()))).await;
    let app_over =
        test::init_service(App::new().service(web::scope("/mcp").service(over.scope()))).await;

    let accepted =
        test::call_service(&app_under, post_request().set_json(&body).to_request()).await;
    let rejected = test::call_service(&app_over, post_request().set_json(&body).to_request()).await;

    assert_ne!(accepted.status().as_u16(), 413);
    assert_eq!(rejected.status().as_u16(), 413);
}
