use std::sync::Arc;

use anyhow::Result;

use crate::{
    configurations::system::Config,
    databases::vector_database::{
        lancedb::LanceDB,
        qdrant::QdrantDatabase,
        traits::{VectorDatabase, VectorDatabaseProvider},
    },
};

/// Dynamically create a vector database
pub async fn create_vector_database(config: &Config) -> Result<Arc<dyn VectorDatabase>> {
    match config.vector_database.provider {
        VectorDatabaseProvider::Qdrant => Ok(Arc::new(QdrantDatabase::new(config).await?)),
        VectorDatabaseProvider::Local => Ok(Arc::new(LanceDB::new(config).await?)),
    }
}
