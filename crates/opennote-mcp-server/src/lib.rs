pub mod traits;

mod requests;
mod responses;
mod service;

use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use anyhow::Result;
use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};

use crate::{service::OpenNoteMCPService, traits::OpenNoteMCPServiceImplementation};

pub async fn run_mcp_server(
    address: &str,
    mcp_implementation: Arc<dyn OpenNoteMCPServiceImplementation>,
) -> Result<()> {
    let service = OpenNoteMCPService::new(mcp_implementation);

    // StreamableHttp service with builder pattern (shared across workers)
    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(move || Ok(service.clone())))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .stateful_mode(true)
        .build();

    HttpServer::new(move || {
        App::new()
            // Your existing routes
            .route("/health", web::get().to(|| async { "OK" }))
            // Mount MCP service at custom path
            .service(web::scope("/mcp").service(http_service.clone().scope()))
    })
    .bind(address)?
    .run()
    .await?;

    Ok(())
}
