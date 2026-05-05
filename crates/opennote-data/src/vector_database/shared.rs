use std::sync::Arc;

use anyhow::{Result, anyhow};

use opennote_models::{
    configurations::system::SystemConfigurations,
    providers::vector_database::VectorDatabaseProvider,
};

use crate::vector_database::{
    lancedb::LanceDB, sqlite::SQLiteVectorDatabase, traits::VectorDatabase,
};

/// Dynamically create a vector database
pub async fn create_vector_database(
    config: &SystemConfigurations,
) -> Result<Arc<dyn VectorDatabase>> {
    let vector_database: Arc<dyn VectorDatabase> = match config.vector_database.provider {
        VectorDatabaseProvider::LanceDB => Arc::new(LanceDB::new(config).await?),
        VectorDatabaseProvider::SQLiteVector => Arc::new(SQLiteVectorDatabase::new(config).await?),
        _ => {
            return Err(anyhow!("Not a supported vector database at the moment"));
        }
    };
    Ok(vector_database)
}
