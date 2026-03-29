use anyhow::Result;
use async_trait::async_trait;

use crate::database::metadata::MetadataSettings;

/// it defines methods for managing metadata
#[async_trait]
pub trait MetadataManagement {
    async fn get_metadata_settings(&self) -> Result<MetadataSettings>;
}
