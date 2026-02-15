use anyhow::Result;
use sqlx::{SqliteConnection, SqlitePool};

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// It will load the existing database, otherwise it will create a new one
    pub async fn new(connection_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(connection_url).await?;

        Ok(Self { pool })
    }
    
    /// Perform upgrades to the existing database
    pub async fn migrate(connection: &SqliteConnection) -> Result<()> {
        Ok(())
    }
}
