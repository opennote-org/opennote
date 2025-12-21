use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::models::ImportTaskIntermediate;

#[async_trait]
pub trait Connector {
    async fn get_intermediate(artifact: Value) -> Result<ImportTaskIntermediate>;
}