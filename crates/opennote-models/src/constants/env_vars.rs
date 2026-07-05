use std::collections::HashMap;

use anyhow::{Result, anyhow};

/*
 * Environment Variable Names
 */
pub const DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME: &str = "DEFAULT_SQLITE_DATA_FOLDER_NAME";
pub const SERVER_PASSWORD_ENV_VAR_NAME: &str = "SERVER_PASSWORD";

/// Environment variable names that need to be initialized on startup
pub const STARTUP_ENVIRONMENT_VARIABLES_FOR_SERVER: [&str; 1] =
    [DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME];
pub const STARTUP_ENVIRONMENT_VARIABLES_FOR_DESKTOP: [&str; 1] =
    [DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME];

/// Load an environment variable, but this function will panic if it does not exist
pub fn load_environment_variable(environment_variable_name: &str) -> String {
    std::env::var(environment_variable_name)
        .unwrap()
        .to_string()
}

/// Set the environment variables that are required on startup.
/// It will return error if one of the required variables is missing.
pub fn set_environment_variables(
    required_vars_names: &[&str],
    environment_variables: HashMap<&str, &str>,
) -> Result<()> {
    for var in required_vars_names {
        match environment_variables.get(var) {
            Some(value) => unsafe {
                std::env::set_var(var, value);
            },
            None => return Err(anyhow!("Environment variable not found: {}", var)),
        }
    }

    Ok(())
}
