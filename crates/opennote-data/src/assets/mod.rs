use std::path::{Path, PathBuf};

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AssetsStorage {
    assets_storage_path: PathBuf,
}

impl AssetsStorage {
    pub fn new(assets_storage_path: impl Into<PathBuf>) -> Self {
        Self {
            assets_storage_path: assets_storage_path.into(),
        }
    }

    /// Create a path that can be directly used to save the asset to the assets storage.
    /// The `storage_reference` looks like this: `<sha256>/<filename>.<extension>`.
    pub fn create_asset_save_path(&self, storage_reference: impl AsRef<Path>) -> PathBuf {
        self.assets_storage_path.join(storage_reference)
    }

    /// Look up for whether the asset exists.
    /// Panic when the permission is not granted.
    pub fn look_up_asset(&self, storage_reference: impl AsRef<Path>) -> bool {
        match self.create_asset_save_path(storage_reference).try_exists() {
            Ok(result) => result,
            Err(error) => panic!("{}", error),
        }
    }

    /// Load an asset into bytes by using a storage reference.
    pub fn load_asset_as_bytes(&self, storage_reference: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = self.create_asset_save_path(storage_reference);
        Ok(std::fs::read(path)?)
    }
}
