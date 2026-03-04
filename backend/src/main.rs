mod api_models;
mod app_state;
mod backup;
mod checkups;
mod configurations;
mod connectors;
mod constants;
mod database;
mod documents;
mod embedder;
mod handlers;
mod identities;
mod mcp;
mod metadata_storage;
mod routes;
mod search;
mod tasks_scheduler;
mod traits;
mod vector_database;
mod initialization;
mod databases;

use actix_web::web::Data;
use anyhow::Result;
use app_state::AppState;
use log::info;

use rmcp_actix_web::transport::StreamableHttpService;
use sqlx::any::install_default_drivers;

use crate::{
    initialization::{initialize_app_state, initialize_backend_api_service, initialize_logger, initialize_mcp_server, load_configurations}, mcp::service::MCPService
};

#[actix_web::main]
async fn main() -> Result<()> {
    // Load configuration first
    let config = load_configurations()?;

    // Initialize logger with config level
    initialize_logger(&config);

    // Install database drivers, otherwise the RelationshipDatabase Connector may fail
    install_default_drivers();
    info!("Default relationship database drivers installed.");

    info!("Starting Actix Web Service...");
    info!(
        "Configuration: Server {}:{}",
        config.server.host, config.server.port
    );

    let app_state: Data<AppState> = initialize_app_state(&config).await?;
    let mcp_service: StreamableHttpService<MCPService> = initialize_mcp_server(&app_state).await?;
    
    initialize_backend_api_service(app_state, mcp_service, &config).await?;
    
    Ok(())
}
