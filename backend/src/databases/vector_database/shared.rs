use std::sync::Arc;

use anyhow::Result;

use crate::{
    configurations::system::Config,
    databases::vector_database::{
        self,
        lancedb::LanceDB,
        qdrant::QdrantDatabase,
        traits::{VectorDatabase, VectorDatabaseProvider},
    },
};

/// Dynamically create a vector database
pub async fn create_vector_database(config: &Config) -> Result<Arc<dyn VectorDatabase>> {
    let vector_database: Arc<dyn VectorDatabase> = match config.vector_database.provider {
        VectorDatabaseProvider::Qdrant => Arc::new(QdrantDatabase::new(config).await?),
        VectorDatabaseProvider::Local => Arc::new(LanceDB::new(config).await?),
    };
    Ok(vector_database)
}
