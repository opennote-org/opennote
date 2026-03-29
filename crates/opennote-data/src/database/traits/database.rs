use anyhow::Result;
use async_trait::async_trait;

use crate::database::traits::{
    blocks::Blocks, metadata::MetadataManagement, payloads::Payloads,
};

#[async_trait]
pub trait Database: Blocks + Payloads + MetadataManagement + Send + Sync {
    async fn create_tables(&self) -> Result<()>;
}
