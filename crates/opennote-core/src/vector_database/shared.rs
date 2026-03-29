use std::sync::Arc;

use anyhow::Result;

use opennote_models::{
    configurations::system::Config, providers::vector_database::VectorDatabaseProvider,
};

use crate::vector_database::{lancedb::LanceDB, traits::VectorDatabase};

/// Dynamically create a vector database
pub async fn create_vector_database(config: &Config) -> Result<Arc<dyn VectorDatabase>> {
    let vector_database: Arc<dyn VectorDatabase> = match config.vector_database.provider {
        VectorDatabaseProvider::Local => Arc::new(LanceDB::new(config).await?),
    };
    Ok(vector_database)
}
