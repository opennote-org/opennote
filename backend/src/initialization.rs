use std::{sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    middleware::Logger,
    web::{self, Data},
};
use anyhow::{Context, Result};

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;

use crate::{
    app_state::AppState,
    checkups::{align_embedder_model, handshake_embedding_service},
    configurations::system::Config,
    mcp::service::MCPService,
    routes::configure_routes,
};

pub fn load_configurations() -> Result<Config> {
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
            let database_information = state.database.peek().await?;

            log::info!(
                "Metadata contains {} documents",
                database_information.number_documents
            );
            log::info!(
                "User information storage file contains {} entries",
                database_information.number_users
            );
            log::info!(
                "Backups storage file contains {} entries",
                state.backups_storage.lock().await.backups.len()
            );
            log::info!(
                "Task scheduler has {} registered tasks",
                state.tasks_scheduler.lock().await.registered_tasks.len()
            );
            log::info!(
                "Vector database will connect to {}",
                config.vector_database.base_url
            );

            // Checkups
            match handshake_embedding_service(&config.embedder).await {
                Ok(_) => log::info!("Embedding service is ONLINE"),
                Err(error) => panic!("{}", error),
            }

            match align_embedder_model(&config, &state).await {
                Ok(_) => log::info!("Embedder model alignment completed successfully"),
                Err(e) => {
                    log::error!("Failed to align embedder model: {}", e);
                    std::process::exit(1);
                }
            }

            // Create shared application state
            log::info!("Application state initialized successfully");

            Ok(Data::new(state))
        }
        Err(e) => {
            log::error!("Failed to initialize app state: {}", e);
            std::process::exit(1);
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
        .await;

    Ok(())
}
