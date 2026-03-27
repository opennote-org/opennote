use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::join;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::configurations::system::{Config, VectorDatabaseConfig};
use crate::databases::database::traits::blocks::BlockQuery;
use crate::databases::database::traits::database::Database;
use crate::databases::search::{keyword::KeywordSearch, semantic::SemanticSearch};
use crate::embedder::vectorize;
use crate::embedders::entry::EmbedderEntry;
use crate::models::payload::Payload;

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
    async fn create_index(&self, index: &str, dimensions: usize) -> Result<()>;

    /// Required for adding new payloads to the database
    async fn create_entries(&self, index: &str, payloads: Vec<Payload>) -> Result<()>;

    async fn delete_entries(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        correspondent_ids: &Vec<Uuid>,
    ) -> Result<()>;

    async fn get_entries(&self, correspondent_ids: &Vec<Uuid>) -> Result<Vec<Payload>>;

    /// Implement this to get `reindex_documents` automatically implemented
    /// The definition of `index` varies by vector databases
    /// In Qdrant, this is called a `collection` while in LanceDB, this is
    /// a `table`
    async fn delete_index(&self, index: &str) -> Result<()>;

    /// Required for reindex features
    async fn reindex_documents(
        &self,
        configuration: &Config,
        database: &Arc<dyn Database>,
        embedder_entry: &EmbedderEntry,
    ) -> Result<()> {
        let chunks = database.read_blocks(&BlockQuery::All).await?;

        let results = join(
            self.reset_index(
                &configuration.vector_database.index,
                configuration.embedder.dimensions,
            ),
            vectorize(&configuration.embedder, chunks, embedder_entry),
        )
        .await;

        results.0?;
        let chunks = results.1?;

        let results = join(
            database.update_blocks(chunks.clone()),
            self.create_entries(&configuration.vector_database.index, chunks),
        )
        .await;

        results.0?;
        results.1?;

        Ok(())
    }

    async fn reset_index(&self, index: &str, dimensions: usize) -> Result<()> {
        self.delete_index(index).await?;
        self.create_index(index, dimensions).await?;

        Ok(())
    }

    /// Whether the vector database properly hosts the data
    async fn validate_data_integrity(
        &self,
        vector_database_config: &VectorDatabaseConfig,
    ) -> Result<bool>;
}
