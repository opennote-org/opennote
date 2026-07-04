use std::collections::HashMap;

use anyhow::{Result, anyhow};

/*
 * Shared between different ends (desktop, server, etc)
 */
pub const CONFIGURATIONS_FILE_NAME: &str = "configurations.json";
pub const DATA_STORAGE_FOLDER_NAME: &str = "data";
pub const VECTOR_DATABASE_FILENAME: &str = "vector_database";
pub const SQLITE_VECTOR_DATABASE_FILE_EXTENSION: &str = "sqlite";

/// Environment variables that need to be initialized on startup
pub const DEFAULT_SQLITE_DATA_FOLDER_NAME: &str = "DEFAULT_SQLITE_DATA_FOLDER_NAME";
pub const STARTUP_ENVIRONMENT_VARIABLES: [&str; 1] = [DEFAULT_SQLITE_DATA_FOLDER_NAME];

/*
 * Desktop Only
 */
pub const APP_DATA_FOLDER_NAME: &str = "opennote";
pub const LOCAL_SERVER_NAME: &str = "local";

/*
 * Server Only
 */
pub const SERVER_DATA_FOLDER_NAME: &str = "opennote_server";
pub const ROOT_ENDPOINT: &str = "/api/v1";
pub const READ_WORKSPACE_BLOCKS_ENDPOINT: &str = "/read_workspace_blocks";
pub const CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/create_blocks_in_workspace";
pub const DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/delete_blocks_in_workspace";
pub const UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/update_blocks_in_workspace";
pub const SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/search_blocks_in_workspace";

/// Load an environment variable, but this function will panic if it does not exist
pub fn load_environment_variable(environment_variable_name: &str) -> String {
    std::env::var(environment_variable_name)
        .unwrap()
        .to_string()
}

/// Set the environment variables that are required on startup.
/// It will return error if one of the required variables is missing.
pub fn set_environment_variables(environment_variables: HashMap<&str, &str>) -> Result<()> {
    for var in STARTUP_ENVIRONMENT_VARIABLES {
        match environment_variables.get(var) {
            Some(value) => unsafe {
                std::env::set_var(var, value);
            },
            None => return Err(anyhow!("Environment variable not found: {}", var)),
        }
    }

    Ok(())
}
