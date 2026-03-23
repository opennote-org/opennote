use std::sync::Arc;

use anyhow::Result;

use crate::{
    configurations::system::Config,
    databases::{
        database::{
            filters::get_document_chunks::GetDocumentChunkFilter, shared::create_database,
            traits::database::Database,
        },
        vector_database::{shared::create_vector_database, traits::VectorDatabase},
    },
};

/// At the moment, it only abstracts database-related logics of documents and chunks.
///
/// Users, collections, configurations and on are not yet needed for further abstractions
#[derive(Clone)]
pub struct DatabasesLayerEntry {
    pub database: Arc<dyn Database>,
    pub vector_database: Arc<dyn VectorDatabase>,
}

impl DatabasesLayerEntry {
    pub async fn new(config: &Config) -> Result<Self> {
        let vector_database = create_vector_database(config).await?;
        let database = create_database(config, &vector_database).await?;

        Ok(Self {
            database,
            vector_database,
        })
    }

    /// Reover all document chunks from the relational database to the vector database
    pub async fn recover(&self, index: &str, dimensions: usize) -> Result<()> {
        let chunks: Vec<DocumentChunk> = self
            .database
            .get_document_chunks(&GetDocumentChunkFilter {
                ..Default::default()
            })
            .await?;

        self.vector_database.reset_index(index, dimensions).await?;

        self.vector_database
            .add_document_chunks_to_database(index, chunks)
            .await?;

        Ok(())
    }
}
