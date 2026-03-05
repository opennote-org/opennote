use actix_web::cookie::time::UtcDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    databases::database::{self, utilities::parse_timestamp},
    documents::{document_chunk::DocumentChunk, traits::GetId},
};

use super::traits::ValidateDataMutabilitiesForAPICaller;

/// For backward compatibility
/// TODO: remove this
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyDocumentMetadata {
    #[serde(alias = "metadata_id")]
    pub id: String,

    pub created_at: String,

    pub last_modified: String,

    pub collection_metadata_id: String,

    pub title: String,

    pub chunks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    #[serde(alias = "metadata_id")]
    pub id: String,

    pub created_at: String,

    pub last_modified: String,

    pub collection_metadata_id: String,

    pub title: String,

    pub chunks: Vec<DocumentChunk>,
}

impl DocumentMetadata {
    pub fn new(title: String, collection_metadata_id: String) -> Self {
        let now = UtcDateTime::now().to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            last_modified: now,
            collection_metadata_id,
            title,
            chunks: Vec::new(),
        }
    }
}

impl GetId for DocumentMetadata {
    fn get_id(&self) -> &str {
        &self.id
    }
}

impl ValidateDataMutabilitiesForAPICaller for DocumentMetadata {
    fn is_mutated(&self) -> anyhow::Result<()> {
        if !self.chunks.is_empty() {
            return Err(anyhow::anyhow!(
                "Document chunks are immutable to API callers"
            ));
        }

        if !self.created_at.is_empty() {
            return Err(anyhow::anyhow!(
                "Document creation date is immutable to API callers"
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

impl
    From<(
        database::entity::documents::Model,
        Vec<database::entity::document_chunks::Model>,
    )> for DocumentMetadata
{
    fn from(
        mut value: (
            database::entity::documents::Model,
            Vec<database::entity::document_chunks::Model>,
        ),
    ) -> Self {
        value.1.sort_by(|a, b| a.chunk_order.cmp(&b.chunk_order));

        Self {
            id: value.0.id,
            created_at: UtcDateTime::from_unix_timestamp(value.0.created_at)
                .unwrap()
                .to_string(),
            last_modified: UtcDateTime::from_unix_timestamp(value.0.last_modified)
                .unwrap()
                .to_string(),
            collection_metadata_id: value.0.collection_metadata_id,
            title: value.0.title,
            chunks: value.1.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<DocumentMetadata>
    for (
        database::entity::documents::Model,
        Vec<database::entity::document_chunks::Model>,
    )
{
    fn from(value: DocumentMetadata) -> Self {
        let document_model = database::entity::documents::Model {
            id: value.id,
            created_at: parse_timestamp(&value.created_at),
            last_modified: parse_timestamp(&value.last_modified),
            collection_metadata_id: value.collection_metadata_id,
            title: value.title,
        };

        let chunk_models: Vec<database::entity::document_chunks::Model> = value
            .chunks
            .into_iter()
            .enumerate()
            .map(|(index, chunk)| {
                let mut model: database::entity::document_chunks::Model = chunk.into();
                model.chunk_order = index as i64;
                model
            })
            .collect();

        (document_model, chunk_models)
    }
}

impl From<database::entity::documents::Model> for DocumentMetadata {
    fn from(value: database::entity::documents::Model) -> Self {
        Self {
            id: value.id,
            created_at: UtcDateTime::from_unix_timestamp(value.created_at)
                .unwrap()
                .to_string(),
            last_modified: UtcDateTime::from_unix_timestamp(value.last_modified)
                .unwrap()
                .to_string(),
            collection_metadata_id: value.collection_metadata_id,
            title: value.title,
            chunks: Vec::new(),
        }
    }
}

impl From<DocumentMetadata> for database::entity::documents::Model {
    fn from(value: DocumentMetadata) -> Self {
        Self {
            id: value.id,
            collection_metadata_id: value.collection_metadata_id,
            title: value.title,
            created_at: parse_timestamp(&value.created_at),
            last_modified: parse_timestamp(&value.last_modified),
        }
    }
}
