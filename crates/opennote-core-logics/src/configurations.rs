use std::path::{Path, PathBuf};

use anyhow::Result;

use opennote_models::constants::{
    APP_DATA_FOLDER_NAME, DATA_STORAGE_FOLDER_NAME, SERVER_DATA_FOLDER_NAME,
};

#[derive(Debug, Copy, Clone)]
pub enum ApplicationType {
    Desktop,
    Server,
}

/// Get the configuration folder path.
/// This function will panic out if no config directory was found.
pub fn get_configuration_folder_path(application_type: ApplicationType) -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        return match application_type {
            ApplicationType::Desktop => config_dir.join(APP_DATA_FOLDER_NAME),
            ApplicationType::Server => config_dir.join(SERVER_DATA_FOLDER_NAME),
        };
    }

    panic!("No config directory was found in this system")
}

/// This is only available to the desktop for now.
pub fn get_remote_servers_configurations_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        return config_dir.join(APP_DATA_FOLDER_NAME);
    }

    panic!("No config directory was found in this system")
}

pub fn create_required_folders(config_directory: &Path) -> Result<()> {
    std::fs::create_dir_all(config_directory)?;
    std::fs::create_dir_all(config_directory.join(DATA_STORAGE_FOLDER_NAME))?;
    Ok(())
}
