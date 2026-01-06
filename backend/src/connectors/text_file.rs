use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{models::ImportTaskIntermediate, traits::Connector};

#[derive(Debug, Clone)]
pub struct TextFileConnector;

#[async_trait]
impl Connector for TextFileConnector {
    async fn get_intermediate(artifact: Value) -> Result<ImportTaskIntermediate> {
        let artifact = artifact.as_str().unwrap().to_string();
        let mut title = String::new();
        for line in artifact.lines() {
            title = line.to_string();
            break;
        }
            
        Ok(
            ImportTaskIntermediate {
                title,
                content: artifact,
            }
        )
    }
}