use std::sync::Arc;

use anyhow::Result;

use crate::{
    configurations::system::{Config, DatabaseConfig, VectorDatabaseConfig},
    databases::{
        database::{shared::create_database, traits::database::Database},
        vector_database::{shared::create_vector_database, traits::VectorDatabase},
    },
    documents::document_metadata::DocumentMetadata,
};

/// At the moment, it only abstracts database-related logics of documents and chunks.
/// 
/// Users, collections, configurations and on are not yet needed for further abstractions
#[derive(Debug)]
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
        database: &DatabaseConfig,
        vector_database: &VectorDatabaseConfig,
        documents: Vec<DocumentMetadata>,
    ) -> Result<()> {
        Ok(())
    }
    
    pub async fn delete_documents(
        &self, 
        database: &DatabaseConfig,
        vector_database: &VectorDatabaseConfig,
        document_metadata_ids: &Vec<String>,
    ) -> Result<Vec<DocumentMetadata>> {
        Ok(())
    }
    
    pub async fn update_documents(
        database: &DatabaseConfig,
        vector_database: &VectorDatabaseConfig,
        documents: Vec<DocumentMetadata>,
    ) -> Result<()> {
        Ok(())
    }
    
    pub async fn get_documents(
        database: &DatabaseConfig,
        vector_database: &VectorDatabaseConfig,
        document_metadata_ids: &Vec<String>
    ) -> Result<Vec<DocumentMetadata>> {
        Ok(())
    }
    
    pub async fn reindex(
        database: &DatabaseConfig,
        vector_database: &VectorDatabaseConfig,
        scope: &ReindexScope,
    ) -> Result<()> {
        Ok(())
    }
}
