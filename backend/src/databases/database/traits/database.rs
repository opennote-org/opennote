use anyhow::Result;
use async_trait::async_trait;

use crate::databases::database::traits::{blocks::Blocks, metadata::MetadataManagement};

#[async_trait]
pub trait Database: Blocks + MetadataManagement + Send + Sync {
    async fn create_tables(&self) -> Result<()>;
}
