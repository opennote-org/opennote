use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use rmcp_actix_web::transport::{LocalSessionManager, StreamableHttpService};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // StreamableHttp service with builder pattern (shared across workers)
    let http_service = StreamableHttpService::builder()
        .service_factory(Arc::new(|| Ok(MyMcpService::new())))
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
    .bind("127.0.0.1:8080")?
    .run()
    .await?;

    Ok(())
}
