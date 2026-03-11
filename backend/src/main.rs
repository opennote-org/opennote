mod api_models;
mod app_state;
mod checkups;
mod configurations;
mod connectors;
mod constants;
mod databases;
mod documents;
mod embedder;
mod handlers;
mod identities;
mod initialization;
mod mcp;
mod metadata_storage;
mod model_loader;
mod routes;
mod tasks_scheduler;
mod traits;

use actix_web::web::Data;
use anyhow::{Context, Result};
use app_state::AppState;
use log::info;

use rmcp_actix_web::transport::StreamableHttpService;
use sqlx::any::install_default_drivers;

use crate::{
    checkups::{align_embedder_model, align_vector_database, handshake_embedding_service},
    initialization::{
        initialize_app_state, initialize_backend_api_service, initialize_local_model,
        initialize_logger, initialize_mcp_server, load_configurations,
    },
    mcp::service::MCPService,
};

#[actix_web::main]
async fn main() -> Result<()> {
    // Load configuration first
    let mut config = load_configurations()?;

    // Initialize logger with config level
    initialize_logger(&config);

    // Initialize local huggingface model
    let _ = initialize_local_model(&mut config).await;

    // Install database drivers, otherwise the RelationshipDatabase Connector may fail
    install_default_drivers();
    info!("Default relationship database drivers installed.");

    info!("Starting Actix Web Service...");
    info!(
        "Configuration: Server {}:{}",
        config.server.host, config.server.port
    );

    let app_state: Data<AppState> = initialize_app_state(&config)
        .await
        .context("App state failed to initialize")?;
    log::info!("Application state initialized successfully");

    // Checkups
    handshake_embedding_service(&config.embedder)
        .await
        .context("Embedding service is OFFLINE")?;
    log::info!("Embedding service is ONLINE");

    align_embedder_model(&config, &app_state)
        .await
        .context(format!(
            "Embedding model {} failed to align",
            config.embedder.dimensions
        ))?;
    log::info!("Embedder model alignment completed successfully");

    align_vector_database(&config, &app_state)
        .await
        .context(format!(
            "Vector database {} failed to align",
            config.vector_database.provider
        ))?;
    log::info!("Vector database alignment completed successfully");

    let mcp_service: StreamableHttpService<MCPService> = initialize_mcp_server(&app_state).await?;

    initialize_backend_api_service(app_state, mcp_service, &config).await?;

    Ok(())
}
