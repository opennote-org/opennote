use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    configurations::system::Config,
    database::{sqlite::SQLiteDatabase, traits::Database},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseProvider {
    SQLite,
}

/// Dynamically create a vector database
pub async fn create_database(config: &Config) -> Result<Arc<dyn Database>> {
    match config.database.provider {
        DatabaseProvider::SQLite => Ok(Arc::new(
            SQLiteDatabase::new(&config.database.connection_url).await?,
        )),
    }
}
