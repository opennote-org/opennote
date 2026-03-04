use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    configurations::system::Config,
    databases::{database::{sqlite::SQLiteDatabase, traits::database::Database}, vector_database::traits::VectorDatabase},
    metadata_storage::MetadataStorage,
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseProvider {
    #[serde(rename = "sqlite")]
    SQLite,
}

/// Dynamically create a vector database
pub async fn create_database(
    config: &Config,
    vector_database: &Arc<dyn VectorDatabase>,
) -> Result<Arc<dyn Database>> {
    match config.database.provider {
        DatabaseProvider::SQLite => {
            // Check if there is a database already first
            let is_database_exist =
                SQLiteDatabase::is_database_exist(&config.database.connection_url).await;

            let is_metadata_storage_exist =
                MetadataStorage::is_metadata_storage_exist(&config.metadata_storage.path);

            let database = Arc::new(SQLiteDatabase::new(&config.database.connection_url).await?);

            database.create_tables().await?;

            // Migrate if the database originally didn't exist AND
            // the metadata storage exists
            if !is_database_exist && is_metadata_storage_exist {
                log::info!("Database does not exist! Try migrating...");
                database
                    .migrate(
                        &config.metadata_storage.path,
                        &config.identities_storage.path,
                        &vector_database,
                    )
                    .await?;
            }

            Ok(database)
        }
    }
}
