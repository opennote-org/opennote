use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::configurations::system::{Config, EmbedderConfig, VectorDatabaseConfig};
use crate::documents::document_chunk::DocumentChunk;
use crate::databases::search::{keyword::KeywordSearch, semantic::SemanticSearch};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorDatabaseProvider {
    Qdrant,
    Local,
}

impl Display for VectorDatabaseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => f.write_str("Built-in"),
            Self::Qdrant => f.write_str("Qdrant"),
        }
    }
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
