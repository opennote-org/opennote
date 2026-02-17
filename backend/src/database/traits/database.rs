use anyhow::Result;
use async_trait::async_trait;

use crate::database::traits::{identities::Identities, metadata::MetadataManagement};

#[async_trait]
pub trait Database: MetadataManagement + Identities + Send + Sync {
    async fn migrate(&self, path: &str) -> Result<()>;
}
