pub mod database;
pub mod search;
pub mod vector_database;

use std::sync::Arc;

use anyhow::Result;
use futures::future::join;
use uuid::Uuid;

use opennote_models::{
    block::Block,
    configurations::system::{SystemConfigurations, VectorDatabaseConfig},
};

use crate::{
    database::{
        enums::{BlockQuery, PayloadQuery},
        shared::create_database,
        traits::database::Database,
    },
    vector_database::{shared::create_vector_database, traits::VectorDatabase},
};

/// At the moment, it only abstracts database-related logics of documents and chunks.
///
/// Users, collections, configurations and on are not yet needed for further abstractions
#[derive(Clone)]
pub struct Databases {
    pub database: Arc<dyn Database>,
    pub vector_database: Arc<dyn VectorDatabase>,
}

impl Databases {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        let vector_database = create_vector_database(config).await?;
        let database = create_database(config).await?;

        Ok(Self {
            database,
            vector_database,
        })
    }

    pub async fn create_blocks(&self, num_blocks: usize) -> Result<Vec<Block>> {
        self.database.create_blocks(num_blocks).await
    }

    pub async fn read_blocks(&self, filter: &BlockQuery) -> Result<Vec<Block>> {
        self.database.read_blocks(filter).await
    }

    pub async fn update_blocks(&self, blocks: Vec<Block>) -> Result<()> {
        self.database.update_blocks(blocks).await
    }

    pub async fn delete_blocks(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        block_ids: Vec<Uuid>,
    ) -> Result<()> {
        let payloads = self
            .database
            .delete_payloads(&PayloadQuery::ByBlockIds(block_ids.clone()))
            .await?;

        let (delete_blocks_result, delete_entries_results) = join(
            self.database.delete_blocks(block_ids),
            self.vector_database.delete_entries(
                vector_database_config,
                &payloads.into_iter().map(|item| item.id).collect(),
            ),
        )
        .await;

        delete_blocks_result?;
        delete_entries_results?;

        Ok(())
    }

    /// Reover all document chunks from the relational database to the vector database
    pub async fn recover(&self, index: &str, dimensions: usize) -> Result<()> {
        let payloads = self.database.read_payloads(&PayloadQuery::All).await?;

        self.vector_database.reset_index(index, dimensions).await?;

        self.vector_database.create_entries(index, payloads).await?;

        Ok(())
    }
}
