use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::configurations::system::{Config, VectorDatabaseConfig};
use crate::databases::database::traits::database::Database;
use crate::databases::search::{keyword::KeywordSearch, semantic::SemanticSearch};
use crate::documents::document_chunk::DocumentChunk;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorDatabaseProvider {
    Qdrant,
    Local,
}

impl Display for VectorDatabaseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => f.write_str("local"),
            Self::Qdrant => f.write_str("qdrant"),
        }
    }
}

#[async_trait]
pub trait VectorDatabase: Send + Sync + SemanticSearch + KeywordSearch {
    /// Actions of creating an index or a collection for a vector database
    /// In Qdrant, this is called creating a collection. 
    /// In LanceDB, this is called creating a table. 
    async fn create_vector_database(&self, configuration: &Config) -> Result<()>;

    /// Required for adding chunk data to the database
    async fn add_document_chunks_to_database(
        &self,
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
    async fn reindex_documents(
        &self,
        configuration: &Config,
        database: &Arc<dyn Database>,
    ) -> Result<()>;

    /// Whether the vector database properly hosts the data
    async fn validate_data_integrity(
        &self,
        vector_database_config: &VectorDatabaseConfig,
    ) -> Result<bool>;
}
