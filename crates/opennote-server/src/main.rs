pub mod endpoints;
pub mod initialization;
pub mod middlewares;
pub mod routes;

use std::collections::HashMap;

use actix_web::web::Data;
use anyhow::Result;
use log::info;

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::constants::{
    SERVER_DATA_FOLDER_NAME,
    env_vars::{
        DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME, STARTUP_ENVIRONMENT_VARIABLES_FOR_SERVER,
        set_environment_variables,
    },
};

use crate::initialization::{
    initialize_backend_api_service, initialize_logger, load_configurations,
};

#[actix_web::main]
async fn main() -> Result<()> {
    set_environment_variables(
        &STARTUP_ENVIRONMENT_VARIABLES_FOR_SERVER,
        HashMap::from([(
            DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME,
            SERVER_DATA_FOLDER_NAME,
        )]),
    )?;

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
