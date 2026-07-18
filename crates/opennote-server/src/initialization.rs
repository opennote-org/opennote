use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    middleware::{Logger, from_fn},
    web::{Data, PayloadConfig},
};
use anyhow::{Context, Result};

use opennote_bootstrap::ApplicationBootStrap;
use opennote_core_logics::configurations::{
    ApplicationType, create_required_folders, get_configuration_folder_path,
};
use opennote_models::{
    configurations::{Configurations, system::LoggingLevel},
    traits::LoadFromAndSaveToFile,
};

use crate::{middlewares::check_password, routes::configure_routes};

pub fn load_configurations() -> Result<Configurations> {
    let config_path = get_configuration_folder_path(ApplicationType::Server);

    create_required_folders(&config_path)?;

    let configurations = Configurations::load_from_file(&config_path)?;

    log::info!(
        "Configuration at `{}` loaded successfully",
        std::path::PathBuf::from(config_path)
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );

    Ok(configurations)
}

pub fn initialize_logger(config: &Configurations) {
    env_logger::Builder::from_default_env()
        .filter_level(match config.system.logging.level {
            LoggingLevel::Trace => log::LevelFilter::Trace,
            LoggingLevel::Debug => log::LevelFilter::Debug,
            LoggingLevel::Info => log::LevelFilter::Info,
            LoggingLevel::Warn => log::LevelFilter::Warn,
            LoggingLevel::Error => log::LevelFilter::Error,
        })
        .init();
}

pub async fn initialize_backend_api_service(
    bootstrap: Data<ApplicationBootStrap>,
    config: &Configurations,
) -> Result<()> {
    // Start HTTP server
    let bind_address: String = format!(
        "{}:{}",
        config.system.server.host, config.system.server.port
    );
    log::info!("Starting HTTP server on {}", bind_address);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .wrap(from_fn(check_password))
            .app_data(bootstrap.clone())
            // Size limit is 100 MB for now.
            .app_data(PayloadConfig::new(100 * 1024 * 1024))
            .service(configure_routes())
        // .service(web::scope("/mcp").service(mcp_service.clone().scope()))
    });

    // Set number of workers if specified
    log::info!("Using {} worker threads", config.system.server.workers);

    server
        .workers(config.system.server.workers)
        .bind(&bind_address)
        .with_context(|| format!("Failed to bind to {}", bind_address))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .run()
        .await?;

    Ok(())
}
