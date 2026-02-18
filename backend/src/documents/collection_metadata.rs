use actix_web::cookie::time::UtcDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::traits::ValidateDataMutabilitiesForAPICaller;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    #[serde(alias = "metadata_id")]
    pub id: String,

    pub created_at: String,

    pub last_modified: String,

    pub title: String,

    // metadata ids of its owned documents
    pub documents_metadata_ids: Vec<String>,
}

impl CollectionMetadata {
    pub fn new(title: String) -> Self {
        let now: String = UtcDateTime::now().to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            last_modified: now,
            title,
            documents_metadata_ids: Vec::new(),
        }
    }
}

impl ValidateDataMutabilitiesForAPICaller for CollectionMetadata {
    fn is_mutated(&self) -> anyhow::Result<()> {
        if !self.documents_metadata_ids.is_empty() {
            return Err(anyhow::anyhow!(
                "Document metadata ids are immutable to API callers"
            ));
        }

        if !self.created_at.is_empty() {
            return Err(anyhow::anyhow!(
                "Collection creation date is immutable to API callers"
            ));
        }

        if !self.last_modified.is_empty() {
            return Err(anyhow::anyhow!(
                "Last modified date is immutable to API callers"
            ));
        }

        Ok(())
    }
}
