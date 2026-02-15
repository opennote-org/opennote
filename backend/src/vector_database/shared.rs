use std::sync::Arc;

use anyhow::Result;

use crate::{
    configurations::system::Config,
    vector_database::{
        qdrant::QdrantDatabase,
        traits::{VectorDatabase, VectorDatabaseKind},
    },
};

/// Dynamically create a vector database
pub async fn create_vector_database(config: &Config) -> Result<Arc<dyn VectorDatabase>> {
    match config.vector_database.kind {
        VectorDatabaseKind::Qdrant => Ok(Arc::new(QdrantDatabase::new(config).await?)),
    }
}
