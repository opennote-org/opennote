use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::join;
use uuid::Uuid;

use opennote_models::{configurations::system::{SystemConfigurations, VectorDatabaseConfig}, payload::Payload};
use opennote_embedder::{entry::EmbedderEntry, vectorization::vectorize};

use crate::{
    database::{enums::BlockQuery, traits::database::Database},
    search::{keyword::KeywordSearch, semantic::SemanticSearch},
};

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
        payload_ids: &Vec<Uuid>,
    ) -> Result<()>;

    /// Implement this to get `reindex_documents` automatically implemented
    /// The definition of `index` varies by vector databases
    /// In Qdrant, this is called a `collection` while in LanceDB, this is
    /// a `table`
    async fn delete_index(&self, index: &str) -> Result<()>;

    /// Required for reindex features
    async fn reindex_documents(
        &self,
        configuration: &SystemConfigurations,
        database: &Arc<dyn Database>,
        embedder_entry: &EmbedderEntry,
    ) -> Result<()> {
        let blocks = database.read_blocks(&BlockQuery::All).await?;
        let payloads: Vec<Payload> = blocks.into_iter().flat_map(|item| item.payloads).collect();

        let results = join(
            self.reset_index(
                &configuration.vector_database.index,
                configuration.embedder.dimensions,
            ),
            vectorize(embedder_entry, &configuration.embedder, payloads),
        )
        .await;

        results.0?;
        let payloads_with_vectors = results.1?;

        let results = join(
            database.update_payloads(payloads_with_vectors.clone()),
            self.create_entries(&configuration.vector_database.index, payloads_with_vectors),
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
