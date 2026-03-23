use anyhow::Result;
use async_trait::async_trait;

use crate::databases::database::traits::metadata::MetadataManagement;

#[async_trait]
pub trait Database: MetadataManagement + Send + Sync {
    async fn create_tables(&self) -> Result<()>;
}
