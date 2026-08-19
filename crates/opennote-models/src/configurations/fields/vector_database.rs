use serde::{Deserialize, Serialize};

use crate::{
    constants::{
        DATA_STORAGE_FOLDER_NAME, SQLITE_VECTOR_DATABASE_FILE_EXTENSION, VECTOR_DATABASE_FILENAME,
        env_vars::{DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME, load_environment_variable},
    },
    providers::vector_database::VectorDatabaseProvider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDatabaseConfig {
    pub provider: VectorDatabaseProvider,
    pub index: String,
    pub base_url: String,
    pub api_key: String,
}

impl Default for VectorDatabaseConfig {
    fn default() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let app_data_folder_name =
                load_environment_variable(DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME);

            // Looks like this but should be an absolute path:
            // ./data
            let mut vector_database_path = config_dir
                .join(app_data_folder_name)
                .join(DATA_STORAGE_FOLDER_NAME)
                .join(VECTOR_DATABASE_FILENAME);

            vector_database_path.add_extension(SQLITE_VECTOR_DATABASE_FILE_EXTENSION);

            return Self {
                provider: VectorDatabaseProvider::SQLiteVector,
                index: "opennote".to_string(),
                base_url: vector_database_path.to_string_lossy().to_string(),
                api_key: "".to_string(),
            };
        }

        panic!("No config directory was found in this system");
    }
}
