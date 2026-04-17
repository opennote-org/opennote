use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::future::{join, join_all};
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, Condition, ConnectOptions, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use uuid::Uuid;

use opennote_models::{block::Block, payload::Payload};

use crate::database::{
    enums::{BlockQuery, PayloadQuery},
    metadata::MetadataSettings,
    traits::{
        blocks::Blocks, database::Database, metadata::MetadataManagement, payloads::Payloads,
        query::DataQueryFilter,
    },
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
        use opennote_entities::metadata_settings;

        match metadata_settings::Entity::find().one(&self.pool).await? {
            Some(result) => Ok(result.into()),
            None => return Err(anyhow!("Metadata settings missed")),
        }
    }
}

#[async_trait]
impl Payloads for SQLiteDatabase {
    async fn create_payloads_with_active_models(
        &self,
        active_models: Vec<opennote_entities::payloads::ActiveModel>,
    ) -> Result<()> {
        use opennote_entities::payloads;

        payloads::Entity::insert_many(active_models)
            .exec_with_returning(&self.pool)
            .await?;

        Ok(())
    }

    async fn create_payloads(&self, payloads: Vec<Payload>) -> Result<()> {
        self.create_payloads_with_active_models(
            payloads
                .into_iter()
                .map(|item| item.to_active_model())
                .collect(),
        )
        .await
    }

    async fn read_payloads(&self, filter: &PayloadQuery) -> Result<Vec<Payload>> {
        use opennote_entities::payloads;

        let conditions = match filter.get_database_filter() {
            None => return Ok(Vec::new()),
            Some(conditions) => conditions,
        };

        let payload_models = payloads::Entity::find()
            .filter(conditions)
            .all(&self.pool)
            .await?;

        Ok(payload_models
            .into_iter()
            .map(|item| Payload::from(item))
            .collect())
    }

    async fn update_payloads(&self, payloads: Vec<Payload>) -> Result<()> {
        let mut active_models = Vec::new();

        for payload in payloads {
            active_models.push(payload.to_active_model());
        }

        match self
            .update_payloads_with_active_models(active_models.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => {
                // for now, we are assuming the update fails because no entries need
                // to update. Hence, we will create the entries instead
                self.create_payloads_with_active_models(active_models)
                    .await?;
                Ok(())
            }
        }
    }

    async fn update_payloads_with_active_models(
        &self,
        active_models: Vec<opennote_entities::payloads::ActiveModel>,
    ) -> Result<()> {
        use opennote_entities::payloads;

        let mut update_tasks = Vec::new();

        for active_model in active_models {
            update_tasks.push(payloads::Entity::update(active_model).exec(&self.pool));
        }

        let results = join_all(update_tasks).await;

        for result in results {
            result?;
        }

        Ok(())
    }

    async fn delete_payloads(&self, filter: &PayloadQuery) -> Result<Vec<Payload>> {
        use opennote_entities::payloads;

        let conditions = match filter.get_database_filter() {
            None => return Ok(Vec::new()),
            Some(conditions) => conditions,
        };

        let payload_models = payloads::Entity::delete_many()
            .filter(conditions)
            .exec_with_returning(&self.pool)
            .await?;

        Ok(payload_models
            .into_iter()
            .map(|item| Payload::from(item))
            .collect())
    }
}

#[async_trait]
impl Blocks for SQLiteDatabase {
    async fn create_blocks(&self, blocks: Vec<Block>) -> Result<Vec<Block>> {
        use opennote_entities::blocks;

        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let active_blocks_payloads_pairs = Block::to_active_models(blocks.clone());
        let mut insert_blocks_tasks = Vec::new();
        let mut payloads_to_insert = Vec::new();

        for (active_block_model, active_payload_model) in active_blocks_payloads_pairs {
            payloads_to_insert.extend(active_payload_model);
            insert_blocks_tasks.push(blocks::Entity::insert(active_block_model).exec(&self.pool));
        }

        let block_update_results = join_all(insert_blocks_tasks).await;
        for result in block_update_results {
            result?;
        }

        self.create_payloads_with_active_models(payloads_to_insert.clone())
            .await?;

        Ok(blocks)
    }

    async fn read_block_path(&self, block_id: Uuid) -> Result<Vec<Block>> {
        let mut path = Vec::new();
        let mut block_id = Some(block_id);

        loop {
            match block_id {
                Some(id) => {
                    let model = self.read_blocks(&BlockQuery::ByIds(vec![id])).await?;

                    if !model.is_empty() {
                        block_id = model[0].parent_id;
                        path.extend(model);
                    }
                }
                None => break,
            }
        }

        path.reverse();
        Ok(path)
    }

    async fn read_blocks(&self, filter: &BlockQuery) -> Result<Vec<Block>> {
        use opennote_entities::blocks;
        use opennote_entities::payloads;

        let conditions = match filter {
            BlockQuery::All => Condition::all(),
            BlockQuery::Root => Condition::any().add(blocks::Column::ParentId.is_null()),
            BlockQuery::ByIds(ids) => {
                if ids.is_empty() {
                    return Ok(vec![]);
                }

                Condition::any()
                    .add(blocks::Column::Id.is_in(ids.iter().map(|item| item.to_string())))
            }
            BlockQuery::ChildrenOf(ids) => {
                if ids.is_empty() {
                    return Ok(vec![]);
                }

                Condition::any()
                    .add(blocks::Column::ParentId.is_in(ids.iter().map(|item| item.to_string())))
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

    /// TODO: pay attention to the payload updates & creations
    async fn update_blocks(&self, blocks: Vec<Block>) -> Result<()> {
        use opennote_entities::blocks;

        if blocks.is_empty() {
            return Ok(());
        }

        let block_ids: Vec<Uuid> = blocks.iter().map(|item| item.id).collect();
        let active_blocks_payloads_pairs = Block::to_active_models(blocks);
        let mut update_blocks_tasks = Vec::new();
        let mut payloads_to_update = Vec::new();

        for (active_block_model, active_payload_model) in active_blocks_payloads_pairs {
            payloads_to_update.extend(active_payload_model);
            update_blocks_tasks.push(
                blocks::Entity::update_many()
                    .set(active_block_model)
                    .filter(blocks::Column::Id.is_in(block_ids.clone()))
                    .exec(&self.pool),
            );
        }

        let (payload_update_result, block_update_results) = join(
            self.update_payloads_with_active_models(payloads_to_update.clone()),
            join_all(update_blocks_tasks),
        )
        .await;

        match payload_update_result {
            Ok(_) => {}
            Err(_) => {
                self.create_payloads_with_active_models(payloads_to_update)
                    .await?
            }
        }

        for result in block_update_results {
            result?;
        }

        Ok(())
    }

    async fn delete_blocks(&self, block_ids: Vec<Uuid>) -> Result<()> {
        use opennote_entities::blocks;

        if block_ids.is_empty() {
            return Ok(());
        }

        blocks::Entity::delete_many()
            .filter_by_ids(block_ids)
            .exec_with_returning(&self.pool)
            .await?;

        Ok(())
    }
}
