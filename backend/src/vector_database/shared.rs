use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use crate::{
    configurations::system::Config,
    vector_database::{
        qdrant::QdrantDatabase,
        traits::{VectorDatabase, VectorDatabaseKind},
    },
};

/// Dynamically create a vector database
pub async fn create_vector_database(config: &Config) -> Result<Arc<Mutex<dyn VectorDatabase>>> {
    match config.database.kind {
        VectorDatabaseKind::Qdrant => Ok(Arc::new(Mutex::new(QdrantDatabase::new(config).await?))),
    }
}
