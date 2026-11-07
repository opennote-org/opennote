use serde::{Deserialize, Serialize};

use crate::constants::{
    APP_DATA_FOLDER_NAME, DATA_STORAGE_FOLDER_NAME, DEFAULT_ASSETS_FOLDER_NAME,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct AssetsConfigurations {
    /// Indicate where to store assets like pasted images.
    pub assets_storage_path: String,
}

impl Default for AssetsConfigurations {
    fn default() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let assets_storage_path = config_dir
                // This is not a sqlite database,
                // hence it does not use the way sqlite does to load the folder name.
                .join(APP_DATA_FOLDER_NAME)
                .join(DATA_STORAGE_FOLDER_NAME)
                .join(DEFAULT_ASSETS_FOLDER_NAME)
                .to_string_lossy()
                .to_string();

            return Self {
                assets_storage_path,
            };
        }

        panic!("No config directory exists")
    }
}
