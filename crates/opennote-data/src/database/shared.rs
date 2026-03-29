use std::sync::Arc;

use anyhow::Result;

use opennote_models::{configurations::system::Config, providers::database::DatabaseProvider};

use crate::{
    database::{sqlite::SQLiteDatabase, traits::database::Database},
};

/// Dynamically create a vector database
pub async fn create_database(
    config: &Config,
) -> Result<Arc<dyn Database>> {
    match config.database.provider {
        DatabaseProvider::SQLite => {
            let database = Arc::new(SQLiteDatabase::new(&config.database.connection_url).await?);

            database.create_tables().await?;

            Ok(database)
        }
    }
}
