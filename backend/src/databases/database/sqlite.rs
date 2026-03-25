use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, Condition, ConnectOptions, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use uuid::Uuid;

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

        Ok(block
            .into_iter()
            .map(|item| Block::from_model(item, vec![]))
            .collect())
    }

    async fn read_blocks(&self, filter: &BlockQuery) -> Result<Vec<Block>> {
        use crate::entity::blocks;
        use crate::entity::payloads;

        let conditions = match filter {
            BlockQuery::Root => Condition::any().add(blocks::Column::ParentId.is_null()),
            BlockQuery::ByIds(ids) => {
                if ids.is_empty() {
                    return Ok(vec![]);
                }

                Condition::any().add(blocks::Column::Id.is_in(ids))
            }
            BlockQuery::ChildrenOf(ids) => {
                if ids.is_empty() {
                    return Ok(vec![]);
                }

                Condition::any().add(blocks::Column::ParentId.is_in(ids))
            }
        };

        let all_blocks_payloads_pairs = blocks::Entity::find()
            .find_with_related(payloads::Entity)
            .filter(conditions)
            .all(&self.pool)
            .await?;

        let blocks: Vec<Block> = Block::from_models(all_blocks_payloads_pairs);

        match filter {
            BlockQuery::ChildrenOf(_) => {
                let mut children: Vec<Block> = blocks;
                let mut current_level_ids: Vec<Uuid> =
                    children.iter().map(|item| item.id).collect();

                // We keep getting the children blocks until we no longer get one
                while !current_level_ids.is_empty() {
                    let conditions =
                        Condition::any().add(blocks::Column::ParentId.is_in(current_level_ids));

                    let all_blocks_payloads_pairs = blocks::Entity::find()
                        .find_with_related(payloads::Entity)
                        .filter(conditions)
                        .all(&self.pool)
                        .await?;

                    if all_blocks_payloads_pairs.is_empty() {
                        break;
                    }

                    let converted_blocks: Vec<Block> =
                        Block::from_models(all_blocks_payloads_pairs);
                    current_level_ids = converted_blocks.iter().map(|item| item.id).collect();
                    children.extend(converted_blocks);
                }

                Ok(children)
            }
            _ => Ok(blocks),
        }
    }

    async fn update_blocks(&self, blocks: Vec<Block>) -> Result<Vec<Block>> {
        use crate::entity::blocks;
        use crate::entity::payloads;

        let mut ids: Vec<Uuid> = blocks.iter().map(|item| item.id).collect();
        let blocks_payloads_model_pairs: Vec<(blocks::Model, Vec<payloads::Model>)> =
            Block::to_models(blocks);

        for block in blocks {
            blocks::Entity::update_many()
                .set(block.to_model())
                .filter(blocks::Column::Id.is_in(ids))
                .exec(&self.pool);
        }

        Ok(())
    }
}
