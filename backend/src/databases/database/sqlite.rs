use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ActiveModelBehavior, ConnectOptions, DatabaseConnection, EntityTrait};

use crate::{
    databases::database::{
        metadata::MetadataSettings,
        traits::{
            blocks::{BlockQuery, Blocks},
            database::Database,
            metadata::MetadataManagement,
        },
    },
    models::block::Block,
};

#[derive(Debug, Clone)]
pub struct SQLiteDatabase {
    pool: DatabaseConnection,
}

#[async_trait]
impl Database for SQLiteDatabase {
    async fn create_tables(&self) -> Result<()> {
        Migrator::up(&self.pool, None).await?;

        Ok(())
    }
}

impl SQLiteDatabase {
    /// It will load the existing database, otherwise it will create a new one
    pub async fn new(connection_url: &str) -> Result<Self> {
        // Ensure that the directory exists
        if let Some(path) = connection_url.strip_prefix("sqlite://") {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // sea-orm will create file when it does not exist,
        // therefore, we don't need to do a manual check like we did when
        // using sqlx
        let mut options = ConnectOptions::new(connection_url);
        options.map_sqlx_sqlite_opts(|options| {
            options
                .busy_timeout(Duration::from_secs(5))
                .journal_mode(sea_orm::sqlx::sqlite::SqliteJournalMode::Wal)
        });
        options.sqlx_logging(false);

        let pool = sea_orm::Database::connect(options).await?;

        Ok(Self { pool })
    }

    pub async fn is_database_exist(connection_string: &str) -> bool {
        let mode_trimed = match connection_string.rfind("?") {
            Some(result) => &connection_string[..result],
            None => connection_string,
        };

        let start_trimed = mode_trimed.trim_start_matches("sqlite://");

        match std::fs::exists(start_trimed) {
            Ok(result) => result,
            Err(_) => false,
        }
    }
}

#[async_trait]
impl MetadataManagement for SQLiteDatabase {
    async fn get_metadata_settings(&self) -> Result<MetadataSettings> {
        use crate::entity::metadata_settings;

        match metadata_settings::Entity::find().one(&self.pool).await? {
            Some(result) => Ok(result.into()),
            None => return Err(anyhow!("Metadata settings missed")),
        }
    }
}

#[async_trait]
impl Blocks for SQLiteDatabase {
    async fn create_blocks(&self, num_blocks: usize) -> Result<Vec<Block>> {
        use crate::entity::blocks::{ActiveModel, Entity as BlockEntity};

        let blocks_active_models: Vec<ActiveModel> = (0..num_blocks)
            .into_iter()
            .map(|_| ActiveModel::new())
            .collect();

        let block = BlockEntity::insert_many(blocks_active_models)
            .exec_with_returning(&self.pool)
            .await?;

        Ok(block.into_iter().map(|item| item.into()).collect())
    }

    async fn read_blocks(filter: &BlockQuery) -> Result<Vec<Block>> {
        Ok(())
    }
}
