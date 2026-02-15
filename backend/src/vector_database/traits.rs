use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::configurations::system::{Config, VectorDatabaseConfig, EmbedderConfig};
use crate::documents::{document_chunk::DocumentChunk, document_metadata::DocumentMetadata};
use crate::metadata_storage::MetadataStorage;
use crate::search::{keyword::KeywordSearch, semantic::SemanticSearch};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorDatabaseKind {
    Qdrant,
}

#[async_trait]
pub trait VectorDatabase: Send + Sync + SemanticSearch + KeywordSearch {
    /// Required for adding chunk data to the database
    async fn add_document_chunks_to_database(
        &self,
        embedder_config: &EmbedderConfig,
        vector_database_config: &VectorDatabaseConfig,
        chunks: Vec<DocumentChunk>,
    ) -> Result<()>;

    async fn add_document_chunks_to_database_and_metadata_storage(
        &self,
        embedder_config: &EmbedderConfig,
        vector_database_config: &VectorDatabaseConfig,
        chunks: Vec<DocumentChunk>,
        metadata_storage: Arc<Mutex<MetadataStorage>>,
        metadata: DocumentMetadata,
    ) -> Result<String> {
        self.add_document_chunks_to_database(embedder_config, vector_database_config, chunks)
            .await?;

        let metadata_id: String = metadata.id.clone();
        metadata_storage.lock().await.add_document(metadata).await?;

        Ok(metadata_id)
    }

    async fn delete_documents_from_database(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        document_ids: &Vec<String>,
    ) -> Result<()>;

    async fn get_document_chunks(
        &self,
        document_chunks_ids: Vec<String>,
    ) -> Result<Vec<DocumentChunk>>;

    /// Required for reindex features
    async fn reindex_documents(&self, configuration: &Config) -> Result<()>;
}
