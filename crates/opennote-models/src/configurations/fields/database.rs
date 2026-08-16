use serde::{Deserialize, Serialize};

use crate::{
    constants::{
        DATA_STORAGE_FOLDER_NAME,
        env_vars::{DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME, load_environment_variable},
    },
    providers::database::DatabaseProvider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub provider: DatabaseProvider,
    pub connection_url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let app_data_folder_name =
                load_environment_variable(DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME);

            // Looks like this but should be an absolute path:
            // sqlite://./data/database.sqlite?mode=rwc
            let path_to_sqlite = config_dir
                .join(app_data_folder_name)
                .join(DATA_STORAGE_FOLDER_NAME)
                .join("database.sqlite")
                .to_string_lossy()
                .to_string();

            return Self {
                provider: DatabaseProvider::SQLite,
                connection_url: format!("sqlite://{}?mode=rwc", path_to_sqlite),
            };
        }

        panic!("No config directory was found in this system");
    }
}
