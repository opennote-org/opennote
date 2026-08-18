pub mod requests;
pub mod responses;
pub mod traits;
pub mod instructions;

mod service;

use std::sync::Arc;

use actix_web::{App, HttpServer, dev::Server, web};
use anyhow::Result;
use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};

use crate::{service::OpenNoteMCPService, traits::OpenNoteMCPServiceImplementation};

pub fn run_mcp_server(
    address: &str,
    workers: usize,
    mcp_implementation: Arc<dyn OpenNoteMCPServiceImplementation>,
) -> Result<Server> {
    let service = OpenNoteMCPService::new(mcp_implementation);

    // StreamableHttp service with builder pattern (shared across workers)
    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(move || Ok(service.clone())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    Ok(HttpServer::new(move || {
        App::new()
            // Your existing routes
            .route("/health", web::get().to(|| async { "OK" }))
            // Mount MCP service at custom path
            .service(web::scope("/").service(http_service.clone().scope()))
    })
    .workers(workers)
    .bind(address)?
    .run())
}
