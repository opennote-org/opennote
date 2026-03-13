use std::sync::Arc;

use anyhow::Result;

use crate::{
    configurations::system::Config,
    databases::vector_database::{
        local::Local,
        qdrant::QdrantDatabase,
        traits::{VectorDatabase, VectorDatabaseProvider},
    },
};

/// Dynamically create a vector database
pub async fn create_vector_database(config: &Config) -> Result<Arc<dyn VectorDatabase>> {
    let database: Arc<dyn VectorDatabase> = match config.vector_database.provider {
        VectorDatabaseProvider::Qdrant => Arc::new(QdrantDatabase::new(config).await?),
        VectorDatabaseProvider::Local => Arc::new(Local::new(config).await?),
    };
    Ok(database)
}
