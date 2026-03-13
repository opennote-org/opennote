use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::join;
use serde::Deserialize;
use serde::Serialize;

use crate::configurations::system::{Config, VectorDatabaseConfig};
use crate::databases::database::filters::get_document_chunks::GetDocumentChunkFilter;
use crate::databases::database::traits::database::Database;
use crate::databases::search::{keyword::KeywordSearch, semantic::SemanticSearch};
use crate::documents::document_chunk::DocumentChunk;
use crate::embedder::vectorize;

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
    async fn create_index(&self, configuration: &Config) -> Result<()>;

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

    /// Implement this to get `reindex_documents` automatically implemented
    /// The definition of `index` varies by vector databases
    /// In Qdrant, this is called a `collection` while in LanceDB, this is
    /// a `table`
    async fn delete_index(
        &self,
        vector_database_configurations: &VectorDatabaseConfig,
    ) -> Result<()>;

    /// Required for reindex features
    async fn reindex_documents(
        &self,
        configuration: &Config,
        database: &Arc<dyn Database>,
    ) -> Result<()> {
        self.delete_index(&configuration.vector_database).await?;

        let chunks = database
            .get_document_chunks(&GetDocumentChunkFilter {
                ..Default::default()
            })
            .await?;

        let results = join(
            self.create_index(configuration),
            vectorize(&configuration.embedder, chunks),
        )
        .await;

        results.0?;
        let chunks = results.1?;

        let results = join(
            database.update_document_chunks(chunks.clone()),
            self.add_document_chunks_to_database(&configuration.vector_database, chunks),
        )
        .await;

        results.0?;
        results.1?;

        Ok(())
    }

    /// Whether the vector database properly hosts the data
    async fn validate_data_integrity(
        &self,
        vector_database_config: &VectorDatabaseConfig,
    ) -> Result<bool>;
}
