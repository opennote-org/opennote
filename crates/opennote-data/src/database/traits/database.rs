use anyhow::Result;
use async_trait::async_trait;

use crate::database::traits::{blocks::Blocks, payloads::Payloads};

#[async_trait]
pub trait Database: Blocks + Payloads + Send + Sync {
    async fn create_tables(&self) -> Result<()>;
}
