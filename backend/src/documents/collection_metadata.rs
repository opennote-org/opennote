use actix_web::cookie::time::UtcDateTime;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{self, utilities::parse_timestamp},
    documents::document_metadata::DocumentMetadata,
};

use super::traits::ValidateDataMutabilitiesForAPICaller;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    #[serde(alias = "metadata_id")]
    pub id: String,

    pub created_at: String,

    pub last_modified: String,

    pub title: String,

    // metadata ids of its owned documents
    pub documents_metadatas: Vec<DocumentMetadata>,
}

impl CollectionMetadata {
    pub fn new(title: String) -> Self {
        let now: String = UtcDateTime::now().to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            last_modified: now,
            title,
            documents_metadatas: Vec::new(),
        }
    }
}

impl ValidateDataMutabilitiesForAPICaller for CollectionMetadata {
    fn is_mutated(&self) -> anyhow::Result<()> {
        if !self.documents_metadatas.is_empty() {
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

impl Into<database::entity::collections::ActiveModel> for CollectionMetadata {
    fn into(self) -> database::entity::collections::ActiveModel {
        database::entity::collections::ActiveModel {
            id: Set(self.id),
            title: Set(self.title),
            created_at: Set(parse_timestamp(&self.created_at)),
            last_modified: Set(parse_timestamp(&self.last_modified)),
        }
    }
}

impl Into<database::entity::collections::ActiveModel> for &mut CollectionMetadata {
    fn into(self) -> database::entity::collections::ActiveModel {
        database::entity::collections::ActiveModel {
            id: Set(self.id.clone()),
            title: Set(self.title.clone()),
            created_at: Set(parse_timestamp(&self.created_at)),
            last_modified: Set(parse_timestamp(&self.last_modified)),
        }
    }
}

impl
    From<(
        database::entity::collections::Model,
        Vec<database::entity::documents::Model>,
    )> for CollectionMetadata
{
    fn from(
        value: (
            database::entity::collections::Model,
            Vec<database::entity::documents::Model>,
        ),
    ) -> Self {
        Self {
            id: value.0.id,
            created_at: UtcDateTime::from_unix_timestamp(value.0.created_at)
                .unwrap()
                .to_string(),
            last_modified: UtcDateTime::from_unix_timestamp(value.0.last_modified)
                .unwrap()
                .to_string(),
            title: value.0.title,
            documents_metadatas: value.1.into_iter().map(|item| item.into()).collect(),
        }
    }
}
