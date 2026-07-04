pub mod endpoints;
pub mod initialization;
pub mod routes;

use std::collections::HashMap;

use actix_web::web::Data;
use anyhow::Result;
use log::info;

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::constants::{
    DEFAULT_SQLITE_DATA_FOLDER_NAME, SERVER_DATA_FOLDER_NAME, set_environment_variables,
};

use crate::initialization::{
    initialize_backend_api_service, initialize_logger, load_configurations,
};

#[actix_web::main]
async fn main() -> Result<()>{
    set_environment_variables(HashMap::from([(
        DEFAULT_SQLITE_DATA_FOLDER_NAME,
        SERVER_DATA_FOLDER_NAME,
    )]))?;

    // Load configuration first
    let config = load_configurations()?;

    // Initialize logger with config level
    initialize_logger(&config);

    info!("Starting OpenNote Server...");
    info!(
        "Configuration: Server {}:{}",
        config.system.server.host, config.system.server.port
    );

    initialize_backend_api_service(
        Data::new(ApplicationBootStrap::new(&config).await?),
        &config,
    )
    .await?;

    Ok(())
}
