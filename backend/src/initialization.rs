use std::{sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    middleware::Logger,
    web::{self, Data},
};
use anyhow::{Context, Result};
use local_embedded::LocalEmbedder;
use model_downloader::LocalModel;
use model_downloader::{Downloader, HFDownloader};

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;

use crate::{
    app_state::AppState, configurations::system::Config, mcp::service::MCPService,
    routes::configure_routes,
};

pub fn load_configurations() -> Result<Config> {
    // Load configuration first
    let config_path: String =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./config.json".to_string());
    let config: Config = Config::load_from_file(&config_path)?;

    // Validate configuration
    config.validate()?;

    log::info!(
        "Configuration at `{}` loaded successfully",
        std::path::PathBuf::from(config_path)
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );

    Ok(config)
}

pub fn initialize_logger(config: &Config) {
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
}

pub async fn initialize_app_state(config: &Config) -> Result<Data<AppState>> {
    match AppState::new(config.clone()).await {
        Ok(state) => {
            let database_information = state.databases_layer_entry.database.peek().await?;

            log::info!(
                "Metadata contains {} documents and {} collections",
                database_information.number_documents,
                database_information.number_collections
            );
            log::info!(
                "User information storage file contains {} entries",
                database_information.number_users
            );
            log::info!(
                "Task scheduler has {} registered tasks",
                state.tasks_scheduler.lock().await.registered_tasks.len()
            );
            log::info!(
                "Vector database will connect to {}",
                config.vector_database.provider
            );

            Ok(Data::new(state))
        }
        Err(e) => {
            log::error!("Failed to initialize app state: {}", e);
            panic!();
        }
    }
}

pub async fn initialize_mcp_server(
    app_state: &Data<AppState>,
) -> Result<StreamableHttpService<MCPService>> {
    let clone = app_state.clone();
    let mcp_service = StreamableHttpService::builder()
        .service_factory(Arc::new(move || Ok(MCPService::new(clone.clone()))))
        .session_manager(Arc::new(LocalSessionManager::default()))
        .sse_keep_alive(Duration::from_secs(30))
        .build();

    log::info!("MCP service initialized");

    Ok(mcp_service)
}

pub async fn initialize_backend_api_service(
    app_state: Data<AppState>,
    mcp_service: StreamableHttpService<MCPService>,
    config: &Config,
) -> Result<()> {
    // Start HTTP server
    let bind_address: String = format!("{}:{}", config.server.host, config.server.port);
    log::info!("Starting HTTP server on {}", bind_address);

    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .service(configure_routes())
            .service(web::scope("/mcp").service(mcp_service.clone().scope()))
    });

    // Set number of workers if specified
    if let Some(workers) = config.server.workers {
        server = server.workers(workers);
        log::info!("Using {} worker threads", workers);
    }

    server
        .bind(&bind_address)
        .with_context(|| format!("Failed to bind to {}", bind_address))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .run()
        .await?;

    Ok(())
}

#[deprecated(note = "Just for backup purpose")]
pub async fn initialize_local_model(config: &Config) -> Result<()> {
    if config.embedder.provider.trim() == "local" {
        let local_model: LocalModel =
            HFDownloader::download_model(&config.embedder.model, false).await?;
        log::debug!("{}", local_model);
    }

    Ok(())
}

#[deprecated(note = "Just for backup purpose")]
pub fn initialize_local_embedder(config: &Config) -> Result<Option<LocalEmbedder>> {
    let embedder_config = &config.embedder;

    match embedder_config.provider.trim() {
        "local" => Ok(Some(LocalEmbedder::new(&embedder_config.model)?)),
        _ => Ok(None),
    }
}
