mod connectors;
mod database;
mod documents;
mod embedder;
mod handlers;
mod handshake;
mod identities;
mod metadata_storage;
mod routes;
mod tasks_scheduler;
mod traits;
mod utilities;
mod app_state;
mod configurations;
mod search;
mod api_models;
mod handler_operations;

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::{Context, Result};
use app_state::AppState;
use log::{error, info};

use configurations::system::Config;
use routes::configure_routes;
use sqlx::any::install_default_drivers;
use tokio::sync::RwLock;

use crate::handshake::handshake_embedding_service;

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    // Load configuration first
    let config_path: String =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./config.json".to_string());
    let config: Config = match Config::load_from_file(&config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }

    // Initialize logger with config level
    env_logger::Builder::from_default_env()
        .filter_level(match config.logging.level.as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        })
        .init();

    // Install database drivers, otherwise the RelationshipDatabase Connector may fail
    install_default_drivers();
    info!("Default relationship database drivers installed.");

    info!("Starting Actix Web Service...");
    info!(
        "Configuration at `{}` loaded successfully",
        std::path::PathBuf::from(config_path)
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );

    info!(
        "Configuration: Server {}:{}",
        config.server.host, config.server.port
    );

    // Create shared application state
    let app_state = match AppState::new(config.clone()).await {
        Ok(state) => {
            info!(
                "Metadata storage file contains {} documents",
                state.metadata_storage.lock().await.documents.len()
            );
            info!(
                "Task scheduler has {} registered tasks",
                state.tasks_scheduler.lock().await.registered_tasks.len()
            );
            info!("Database will connect to {}", config.database.base_url);
            web::Data::new(RwLock::new(state))
        }
        Err(e) => {
            error!("Failed to initialize app state: {}", e);
            std::process::exit(1);
        }
    };
    
    // Handshakes
    match handshake_embedding_service(&config.embedder).await 
    {
        Ok(_) => info!("Embedding service is ONLINE"),
        Err(error) => panic!("{}", error)
    }

    info!("Application state initialized successfully");

    // Start HTTP server
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    info!("Starting HTTP server on {}", bind_address);

    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .service(configure_routes())
    });

    // Set number of workers if specified
    if let Some(workers) = config.server.workers {
        server = server.workers(workers);
        info!("Using {} worker threads", workers);
    }

    server
        .bind(&bind_address)
        .with_context(|| format!("Failed to bind to {}", bind_address))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .run()
        .await
}
