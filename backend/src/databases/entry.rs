use std::sync::Arc;

use anyhow::Result;
use futures::future::join;

use crate::{
    configurations::system::{Config, VectorDatabaseConfig},
    databases::{
        database::{
            filters::get_document_chunks::GetDocumentChunkFilter, shared::create_database,
            traits::database::Database,
        },
        vector_database::{shared::create_vector_database, traits::VectorDatabase},
    },
    documents::{document_chunk::DocumentChunk, document_metadata::DocumentMetadata},
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

    pub async fn add_documents(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        documents: Vec<DocumentMetadata>,
    ) -> Result<()> {
        let chunks: Vec<DocumentChunk> = documents
            .iter()
            .flat_map(|item| item.chunks.clone())
            .collect();

        let results = join(
            self.database.add_documents(documents),
            self.vector_database
                .add_document_chunks_to_database(vector_database_config, chunks),
        )
        .await;

        results.0?;
        results.1?;

        Ok(())
    }

    pub async fn delete_documents(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        document_metadata_ids: &Vec<String>,
    ) -> Result<Vec<DocumentMetadata>> {
        let results = join(
            self.database.delete_documents(document_metadata_ids),
            self.vector_database
                .delete_documents_from_database(vector_database_config, document_metadata_ids),
        )
        .await;

        results.1?;
        Ok(results.0?)
    }

    pub async fn update_documents(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        documents: Vec<DocumentMetadata>,
    ) -> Result<()> {
        self.vector_database
            .delete_documents_from_database(
                vector_database_config,
                &documents.iter().map(|item| item.id.clone()).collect(),
            )
            .await?;

        let results = join(
            self.database.update_documents(documents.clone()),
            self.vector_database.add_document_chunks_to_database(
                vector_database_config,
                documents.into_iter().flat_map(|item| item.chunks).collect(),
            ),
        )
        .await;

        results.0?;
        results.1?;

        Ok(())
    }

    /// Reover all document chunks from the relational database to the vector database
    pub async fn recover(&self, vector_database_config: &VectorDatabaseConfig) -> Result<()> {
        let chunks: Vec<DocumentChunk> = self
            .database
            .get_document_chunks(&GetDocumentChunkFilter {
                ..Default::default()
            })
            .await?;

        // Only delete all documents when the vector database is not integral
        if self
            .vector_database
            .validate_data_integrity(vector_database_config)
            .await?
        {
            self.vector_database
                .delete_documents_from_database(
                    vector_database_config,
                    &chunks.iter().map(|item| item.id.clone()).collect(),
                )
                .await?;
        }

        self.vector_database
            .add_document_chunks_to_database(vector_database_config, chunks)
            .await?;

        Ok(())
    }
}
