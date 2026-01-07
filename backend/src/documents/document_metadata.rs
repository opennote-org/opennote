use actix_web::cookie::time::UtcDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::traits::ValidateDataMutabilitiesForAPICaller;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub metadata_id: String,
    pub created_at: String,
    pub last_modified: String,
    pub collection_metadata_id: String,
    pub title: String,
    pub chunks: Vec<String>,
}

impl DocumentMetadata {
    pub fn new(title: String, collection_metadata_id: String) -> Self {
        let now = UtcDateTime::now().to_string();
        Self {
            metadata_id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            last_modified: now,
            collection_metadata_id,
            title,
            chunks: Vec::new(),
        }
    }
}

impl ValidateDataMutabilitiesForAPICaller for DocumentMetadata {
    fn is_mutated(&self) -> anyhow::Result<()> {
        if !self.chunks.is_empty() {
            return Err(anyhow::anyhow!("Document chunks are immutable to API callers"));
        }
        
        if !self.created_at.is_empty() {
            return Err(anyhow::anyhow!("Document creation date is immutable to API callers"));
        }
        
        if !self.last_modified.is_empty() {
            return Err(anyhow::anyhow!("Last modified date is immutable to API callers"));
        }
        
        Ok(())
    }
}
