use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use opennote_models::constants::{APP_DATA_FOLDER_NAME, DATA_STORAGE_FOLDER_NAME};

use crate::globals::{assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap};

pub fn get_language_profile(
    bootstrap: &GlobalApplicationBootStrap,
    assets_collection: &AssetsCollection,
) -> Result<HashMap<String, String>> {
    let configurations = bootstrap.get_configurations();

    let language = configurations.user.language.to_string();

    Ok(assets_collection
        .language_profiles
        .get(&language)
        .unwrap()
        .to_owned())
}

pub fn create_required_folders(config_directory: &Path) -> Result<()> {
    std::fs::create_dir_all(config_directory)?;
    std::fs::create_dir_all(config_directory.join(DATA_STORAGE_FOLDER_NAME))?;
    Ok(())
}

/// Get the configuration folder path.
/// This function will panic out if no config directory was found.
pub fn get_configuration_folder_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        let path = config_dir.join(APP_DATA_FOLDER_NAME);
        log::debug!(
            "Configuration directory has been set to: {}",
            path.display()
        );

        path
    } else {
        panic!("No config directory was found in this system")
    }
}

/// Run async codes in sync functions
pub fn run_async_code<F, R>(closure: F) -> R
where
    F: Future<Output = R>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(closure))
}
