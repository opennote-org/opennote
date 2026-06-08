use std::path::{Path, PathBuf};

use anyhow::Result;
use opennote_models::constants::APP_DATA_FOLDER_NAME;

use crate::globals::{
    assets::{AssetsCollection, LanguageProfile},
    bootstrap::GlobalApplicationBootStrap,
};

pub fn get_language_profile(
    bootstrap: &GlobalApplicationBootStrap,
    assets_collection: &AssetsCollection,
) -> Result<LanguageProfile> {
    let handle = tokio::runtime::Handle::current();

    let language = handle.block_on(async move {
        bootstrap
            .0
            .configurations
            .lock()
            .await
            .user
            .language
            .to_string()
    });

    Ok(assets_collection
        .language_profiles
        .get(&language)
        .unwrap()
        .to_owned())
}

pub fn create_required_folders(config_directory: &Path) -> Result<()> {
    std::fs::create_dir_all(config_directory)?;
    Ok(())
}

/// Get the configuration filepath.
/// This function will panic out if no config directory was found.
pub fn get_configuration_filepath() -> PathBuf {
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
