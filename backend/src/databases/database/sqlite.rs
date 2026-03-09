use std::sync::Arc;
use std::time::Duration;

use actix_web::cookie::time::UtcDateTime;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use futures::future::join_all;
use migration::{Migrator, MigratorTrait};
use sea_orm::ActiveValue::Unchanged;
use sea_orm::{ActiveModelBehavior, IntoActiveModel, PaginatorTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
    sea_query::{Expr, OnConflict},
};

use crate::databases::vector_database::traits::VectorDatabase;
use crate::documents::{
    document_chunk::DocumentChunk, traits::ValidateDataMutabilitiesForAPICaller,
};
use crate::traits::LoadAndSave;
use crate::{identities::storage::IdentitiesStorage, metadata_storage::MetadataStorage};

use crate::{
    configurations::user::UserConfigurations,
    databases::database::{
        database_information::DatabaseInformation,
        filters::get_users::GetUserFilter,
        filters::{
            get_collections::GetCollectionFilter, get_document_chunks::GetDocumentChunkFilter,
            get_documents::GetDocumentFilter, traits::GetFilterValidation,
        },
        metadata::MetadataSettings,
        traits::{database::Database, identities::Identities, metadata::MetadataManagement},
        utilities::map_order_by_ids,
        utilities::parse_timestamp,
    },
    documents::{collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata},
    identities::user::User,
};

#[derive(Debug, Clone)]
pub struct SQLiteDatabase {
    pool: DatabaseConnection,
}

#[async_trait]
impl Database for SQLiteDatabase {
    async fn migrate_users(&self, identities_storage: &IdentitiesStorage) -> Result<()> {
        // migrate all users
        use crate::databases::database::entity::users;

        let mut users_to_insert = Vec::new();

        for user in identities_storage.users.iter() {
            let config_json = serde_json::to_value(&user.configuration)
                .context("Failed to serialize user configurations")?;

            let resource_ids = serde_json::to_value(&user.resources)
                .context("Failed to serialize user resource ids")?;

            users_to_insert.push(users::ActiveModel {
                id: Set(user.id.clone()),
                username: Set(user.username.clone()),
                password: Set(user.password.clone()),
                configuration: Set(config_json),
                resource_ids: Set(resource_ids),
                ..Default::default()
            });
        }

        if !users_to_insert.is_empty() {
            users::Entity::insert_many(users_to_insert)
                .on_conflict(
                    OnConflict::column(users::Column::Id)
                        .do_nothing()
                        .to_owned(),
                )
                .exec(&self.pool)
                .await?;
        }

        Ok(())
    }

    async fn migrate_collections(&self, metadata_storage: &MetadataStorage) -> Result<()> {
        // migrate all collection records
        use crate::databases::database::entity::collections;
        let mut collections_to_insert = Vec::new();
        for (_, collection) in metadata_storage.collections.iter() {
            collections_to_insert.push(collections::ActiveModel {
                id: Set(collection.id.to_string()),
                title: Set(collection.title.to_string()),
                created_at: Set(parse_timestamp(&collection.created_at)),
                last_modified: Set(parse_timestamp(&collection.last_modified)),
                ..Default::default()
            });
        }

        collections::Entity::insert_many(collections_to_insert)
            .on_conflict(
                OnConflict::column(collections::Column::Id)
                    .do_nothing()
                    .to_owned(),
            )
            .exec(&self.pool)
            .await?;

        Ok(())
    }

    async fn migrate_documents(
        &self,
        metadata_storage: &MetadataStorage,
        vector_database: &Arc<dyn VectorDatabase>,
    ) -> Result<()> {
        // migrate all document records
        use crate::databases::database::entity::{document_chunks, documents};

        let mut documents_to_insert = Vec::new();
        let mut chunks_to_insert = Vec::new();
        for (_, document) in metadata_storage.documents.iter() {
            let chunks = vector_database
                .get_document_chunks(document.chunks.clone())
                .await?;

            documents_to_insert.push(documents::ActiveModel {
                id: Set(document.id.clone()),
                collection_metadata_id: Set(document.collection_metadata_id.clone()),
                title: Set(document.title.to_string()),
                created_at: Set(parse_timestamp(&document.created_at)),
                last_modified: Set(parse_timestamp(&document.last_modified)),
            });

            // migrate all document chunks
            for (chunk_order, document_chunk) in chunks.into_iter().enumerate() {
                chunks_to_insert.push(document_chunks::ActiveModel {
                    id: Set(document_chunk.id),
                    document_metadata_id: Set(document.id.clone()),
                    collection_metadata_id: Set(document.collection_metadata_id.clone()),
                    content: Set(document_chunk.content),
                    chunk_order: Set(chunk_order as i64),
                    dense_text_vector: Set(
                        serde_json::to_value(document_chunk.dense_text_vector).unwrap()
                    ),
                });
            }
        }

        if !documents_to_insert.is_empty() {
            documents::Entity::insert_many(documents_to_insert)
                .on_conflict(
                    OnConflict::column(documents::Column::Id)
                        .do_nothing()
                        .to_owned(),
                )
                .exec(&self.pool)
                .await?;
        }

        dbg!(&chunks_to_insert);
        if !chunks_to_insert.is_empty() {
            document_chunks::Entity::insert_many(chunks_to_insert)
                .on_conflict(
                    OnConflict::column(document_chunks::Column::Id)
                        .do_nothing()
                        .to_owned(),
                )
                .exec(&self.pool)
                .await?;
        }

        Ok(())
    }

    async fn migrate_metadata_settings(&self, metadata_storage: &MetadataStorage) -> Result<()> {
        // migrate the global settings in metadata storage first
        use crate::databases::database::entity::metadata_settings;
        metadata_settings::Entity::update_many()
            .col_expr(
                metadata_settings::Column::EmbedderModelInUse,
                Expr::value(metadata_storage.embedder_model_in_use.clone()),
            )
            .col_expr(
                metadata_settings::Column::EmbedderModelVectorSizeInUse,
                Expr::value(metadata_storage.embedder_model_vector_size_in_use as i64),
            )
            .filter(metadata_settings::Column::Id.eq(1))
            .exec(&self.pool)
            .await?;

        Ok(())
    }

    /// Perform upgrades to the existing database
    async fn migrate(
        &self,
        metadata_storage_path: &str,
        identities_storage_path: &str,
        vector_database: &Arc<dyn VectorDatabase>,
    ) -> Result<()> {
        let metadata_storage = MetadataStorage::load(metadata_storage_path)?;
        let identities_storage = IdentitiesStorage::load(identities_storage_path)?;

        self.migrate_metadata_settings(&metadata_storage).await?;
        self.migrate_collections(&metadata_storage).await?;
        self.migrate_documents(&metadata_storage, vector_database)
            .await?;
        self.migrate_users(&identities_storage).await?;

        Ok(())
    }

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
impl Identities for SQLiteDatabase {
    async fn create_user(&self, username: String, password: String) -> Result<()> {
        use crate::databases::database::entity::users::*;

        let user = User::new(username, password);

        Entity::insert::<ActiveModel>(user.into())
            .exec(&self.pool)
            .await?;

        Ok(())
    }

    async fn add_users(&self, users: Vec<User>) -> Result<()> {
        if users.is_empty() {
            return Ok(());
        }

        use crate::databases::database::entity::users::*;

        let users: Vec<ActiveModel> = users.into_iter().map(|item| item.into()).collect();

        Entity::insert_many(users).exec(&self.pool).await?;

        Ok(())
    }

    /// This will remove the owning collections from the database too
    async fn delete_users(&self, usernames: Vec<String>) -> Result<Vec<User>> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        use crate::databases::database::entity;

        // delete the users and their user resources
        let mut conditions = sea_orm::Condition::any();
        for username in usernames {
            conditions = conditions.add(entity::users::Column::Username.eq(username));
        }

        let users = entity::users::Entity::delete_many()
            .filter(conditions)
            .exec_with_returning(&self.pool)
            .await?;

        self.delete_collections(
            &users
                .iter()
                .flat_map(|item| {
                    let resource_ids: Vec<String> =
                        serde_json::from_value(item.resource_ids.clone()).unwrap();
                    resource_ids
                })
                .collect(),
        )
        .await?;

        Ok(users.into_iter().map(|item| item.into()).collect())
    }

    async fn validate_user_password(&self, username: &str, password: &str) -> Result<bool> {
        use crate::databases::database::entity::users;

        let result = users::Entity::find_by_username(username)
            .one(&self.pool)
            .await?;

        if let Some(result) = result {
            return Ok(result.password == password);
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    async fn add_authorized_resources(
        &self,
        username: &str,
        resource_ids: Vec<String>,
    ) -> Result<()> {
        use crate::databases::database::entity::users;

        if let Some(user) = users::Entity::find_by_username(username)
            .one(&self.pool)
            .await?
        {
            let mut user: users::ActiveModel = user.into();
            let mut existing_resource_ids: Vec<String> =
                serde_json::from_value(user.resource_ids.take().unwrap())?;

            existing_resource_ids.extend(resource_ids);
            user.resource_ids = Set(serde_json::to_value(&existing_resource_ids)?);

            user.update(&self.pool).await?;

            return Ok(());
        }

        Err(anyhow!("username {} does not exist", username))
    }

    async fn remove_authorized_resources(
        &self,
        username: &str,
        resource_ids: &Vec<String>,
    ) -> Result<()> {
        use crate::databases::database::entity::users;

        if let Some(user) = users::Entity::find_by_username(username)
            .one(&self.pool)
            .await?
        {
            let mut user: users::ActiveModel = user.into();
            let mut existing_resource_ids: Vec<String> =
                serde_json::from_value(user.resource_ids.take().unwrap())?;

            existing_resource_ids.retain(|item| !resource_ids.contains(item));
            user.resource_ids = Set(serde_json::to_value(&existing_resource_ids)?);

            user.update(&self.pool).await?;

            return Ok(());
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    async fn update_user_configurations(
        &self,
        username: &str,
        user_configurations: UserConfigurations,
    ) -> Result<()> {
        use crate::databases::database::entity::users;

        let config_json = serde_json::to_value(&user_configurations)
            .context("user configuration serialization failed")?;

        let user = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.pool)
            .await?;

        if let Some(user) = user {
            let mut user: users::ActiveModel = user.into();
            user.configuration = Set(config_json);
            user.update(&self.pool).await?;

            return Ok(());
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    async fn get_resource_ids_by_username(&self, username: &str) -> Result<Vec<String>> {
        use crate::databases::database::entity::users;

        let user = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.pool)
            .await?;

        if let Some(user) = user {
            let resources: Vec<String> = serde_json::from_value(user.resource_ids)?;
            return Ok(resources);
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    async fn is_user_owning_collections(
        &self,
        username: &str,
        collection_metadata_ids: &[String],
    ) -> Result<bool> {
        use crate::databases::database::entity::users;

        let user = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.pool)
            .await?;

        if let Some(user) = user {
            let resources: Vec<String> = serde_json::from_value(user.resource_ids)
                .context("user resources serialization failed")?;

            for id in collection_metadata_ids {
                if !resources.contains(id) {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        Ok(false)
    }

    async fn get_users(&self, filter: &GetUserFilter) -> Result<Vec<User>> {
        use crate::databases::database::entity::users;

        if filter.is_over_constrained() {
            return Err(anyhow!("only one filter is applicable"));
        }

        if filter.is_empty_filter() {
            // Start filtering
            let users = users::Entity::find().all(&self.pool).await?;

            return Ok(users.into_iter().map(|item| item.into()).collect());
        }

        // Construct a sql filter to the users table
        let mut sql_filter_to_users = sea_orm::Condition::any();

        if !filter.ids.is_empty() {
            sql_filter_to_users = sql_filter_to_users.add(users::Column::Id.is_in(&filter.ids));
        }

        if !filter.usernames.is_empty() {
            sql_filter_to_users =
                sql_filter_to_users.add(users::Column::Username.is_in(&filter.usernames));
        }

        // Start filtering
        let users = users::Entity::find()
            .filter(sql_filter_to_users)
            .all(&self.pool)
            .await?;

        Ok(users.into_iter().map(|item| item.into()).collect())
    }
}

#[async_trait]
impl MetadataManagement for SQLiteDatabase {
    async fn create_collection(&self, title: &str) -> Result<String> {
        use crate::databases::database::entity::collections;

        let collection: collections::ActiveModel =
            CollectionMetadata::new(title.to_string()).into();

        // Clone the id for return before it is consumed
        let collection_id: String = collection.id.clone().take().unwrap();

        collection.insert(&self.pool).await?;

        Ok(collection_id)
    }

    async fn update_collections(
        &self,
        mut collection_metadatas: Vec<CollectionMetadata>,
    ) -> Result<()> {
        use crate::databases::database::entity::collections;

        let mut tasks = Vec::new();
        for metadata in collection_metadatas.iter_mut() {
            metadata.is_mutated()?;
            metadata.last_modified = UtcDateTime::now().to_string();

            tasks.push(async {
                let active_model: collections::ActiveModel = collections::ActiveModel {
                    last_modified: Set(parse_timestamp(&metadata.last_modified)),
                    title: Set(metadata.title.clone()),
                    ..Default::default()
                };
                active_model.update(&self.pool).await?;

                Ok::<_, anyhow::Error>(())
            });
        }

        let results = join_all(tasks).await;
        for result in results {
            result?;
        }

        Ok(())
    }

    async fn update_documents(&self, document_metadatas: Vec<DocumentMetadata>) -> Result<()> {
        use crate::databases::database::entity::documents;

        // Concurrently update the document metadatas
        let mut update_document_metadata_tasks = Vec::new();
        let mut update_chunks_tasks = Vec::new();
        for metadata in document_metadatas.into_iter() {
            update_chunks_tasks.push(self.update_document_chunks(metadata.chunks.clone()));

            let metadata_model: documents::Model = metadata.clone().into();
            let mut metadata_active_model = metadata_model.into_active_model();

            metadata_active_model.collection_metadata_id = Set(metadata.collection_metadata_id);
            metadata_active_model.title = Set(metadata.title);
            metadata_active_model.last_modified =
                Set(parse_timestamp(&UtcDateTime::now().to_string()));

            update_document_metadata_tasks.push(metadata_active_model.update(&self.pool));
        }

        let results = join_all(update_document_metadata_tasks).await;
        for result in results {
            result?;
        }

        if !update_chunks_tasks.is_empty() {
            let results = join_all(update_chunks_tasks).await;
            for result in results {
                result?;
            }
        }

        Ok(())
    }

    async fn get_metadata_settings(&self) -> Result<MetadataSettings> {
        use crate::databases::database::entity::metadata_settings;

        match metadata_settings::Entity::find().one(&self.pool).await? {
            Some(result) => Ok(result.into()),
            None => return Err(anyhow!("Metadata settings missed")),
        }
    }

    async fn update_metadata_settings(&self, settings: MetadataSettings) -> Result<()> {
        use crate::databases::database::entity::metadata_settings;

        metadata_settings::Entity::update(metadata_settings::ActiveModel {
            id: Unchanged(1),
            embedder_model_in_use: Set(settings.embedder_model_in_use),
            embedder_model_vector_size_in_use: Set(
                settings.embedder_model_vector_size_in_use as i64
            ),
            vector_database_in_use: Set(settings.vector_database_in_use),
        })
        .exec(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_collections(
        &self,
        collection_metadata_ids: &Vec<String>,
    ) -> Result<Vec<CollectionMetadata>> {
        use crate::databases::database::entity::collections;
        Ok(collections::Entity::delete_many()
            .filter_by_ids(collection_metadata_ids.clone())
            .exec_with_returning(&self.pool)
            .await?
            .into_iter()
            .map(|item| item.into())
            .collect())
    }

    async fn delete_documents(
        &self,
        document_metadata_ids: &Vec<String>,
    ) -> Result<Vec<DocumentMetadata>> {
        use crate::databases::database::entity::documents;

        Ok(documents::Entity::delete_many()
            .filter_by_ids(document_metadata_ids.clone())
            .exec_with_returning(&self.pool)
            .await?
            .into_iter()
            .map(|item| item.into())
            .collect())
    }

    async fn add_collections(&self, collection_metadatas: Vec<CollectionMetadata>) -> Result<()> {
        use crate::databases::database::entity::collections;

        collections::Entity::insert_many(
            collection_metadatas
                .into_iter()
                .map(|item| item.into())
                .collect::<Vec<collections::ActiveModel>>(),
        )
        .exec(&self.pool)
        .await?;

        Ok(())
    }

    async fn add_documents(&self, document_metadatas: Vec<DocumentMetadata>) -> Result<()> {
        use crate::databases::database::entity::documents;

        let chunks = document_metadatas
            .iter()
            .flat_map(|item| item.chunks.clone())
            .collect();

        let active_model_documents: Vec<documents::ActiveModel> = document_metadatas
            .into_iter()
            .map(|item| {
                let model: documents::Model = item.into();
                model.into_active_model()
            })
            .collect();

        documents::Entity::insert_many(active_model_documents)
            .exec(&self.pool)
            .await?;

        self.add_document_chunks(chunks).await?;

        Ok(())
    }

    async fn update_document_chunks(&self, document_chunks: Vec<DocumentChunk>) -> Result<()> {
        use crate::databases::database::entity::document_chunks;

        let models: Vec<document_chunks::ActiveModel> = document_chunks
            .into_iter()
            .enumerate()
            .map(|(index, item)| document_chunks::ActiveModel {
                id: Unchanged(item.id),
                document_metadata_id: Set(item.document_metadata_id),
                collection_metadata_id: Set(item.collection_metadata_id),
                content: Set(item.content),
                dense_text_vector: Set(serde_json::to_value(item.dense_text_vector).unwrap()),
                chunk_order: Set(index as i64),
            })
            .collect();

        document_chunks::Entity::insert_many(models)
            .on_conflict(
                OnConflict::columns([
                    document_chunks::Column::DocumentMetadataId,
                    document_chunks::Column::ChunkOrder,
                ])
                .update_columns([
                    document_chunks::Column::Content,
                    document_chunks::Column::DocumentMetadataId,
                    document_chunks::Column::CollectionMetadataId,
                    document_chunks::Column::DenseTextVector,
                    document_chunks::Column::ChunkOrder,
                ])
                .to_owned(),
            )
            .exec(&self.pool)
            .await?;

        Ok(())
    }

    async fn add_document_chunks(&self, document_chunks: Vec<DocumentChunk>) -> Result<()> {
        use crate::databases::database::entity::document_chunks;

        let models: Vec<document_chunks::ActiveModel> = document_chunks
            .into_iter()
            .enumerate()
            .map(|(index, item)| document_chunks::ActiveModel {
                id: Set(item.id),
                document_metadata_id: Set(item.document_metadata_id),
                collection_metadata_id: Set(item.collection_metadata_id),
                content: Set(item.content),
                dense_text_vector: Set(serde_json::to_value(item.dense_text_vector).unwrap()),
                chunk_order: Set(index as i64),
            })
            .collect();

        document_chunks::Entity::insert_many(models)
            .exec(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_document_chunks(
        &self,
        document_chunk_ids: &Vec<String>,
    ) -> Result<Vec<DocumentChunk>> {
        use crate::databases::database::entity::document_chunks;

        let chunks = document_chunks::Entity::delete_many()
            .filter_by_ids(document_chunk_ids.clone())
            .exec_with_returning(&self.pool)
            .await?;

        Ok(map_order_by_ids(chunks, document_chunk_ids))
    }

    /// Pass an empty filter to get all chunks
    async fn get_document_chunks(
        &self,
        filter: &GetDocumentChunkFilter,
    ) -> Result<Vec<DocumentChunk>> {
        use crate::databases::database::entity::document_chunks;

        if filter.is_over_constrained() {
            return Err(anyhow!("only one filter is applicable"));
        }

        if filter.is_empty_filter() {
            let chunks = document_chunks::Entity::find().all(&self.pool).await?;
            return Ok(chunks.into_iter().map(|item| item.into()).collect());
        }

        // Construct a sql filter to the table
        let mut sql_filter = sea_orm::Condition::any();

        if !filter.ids.is_empty() {
            sql_filter = sql_filter.add(document_chunks::Column::Id.is_in(&filter.ids));
        }

        if !filter.document_metadata_ids.is_empty() {
            sql_filter = sql_filter.add(
                document_chunks::Column::DocumentMetadataId.is_in(&filter.collection_metadata_ids),
            );
        }

        if !filter.collection_metadata_ids.is_empty() {
            sql_filter = sql_filter.add(
                document_chunks::Column::CollectionMetadataId
                    .is_in(&filter.collection_metadata_ids),
            );
        }

        // Start filtering
        let chunks = document_chunks::Entity::find()
            .filter(sql_filter)
            .all(&self.pool)
            .await?;

        Ok(map_order_by_ids(chunks, &filter.ids))
    }

    /// It gurantees the order of the metadata will follow the input ids order
    async fn get_documents(&self, filter: &GetDocumentFilter) -> Result<Vec<DocumentMetadata>> {
        use crate::databases::database::entity::{document_chunks, documents};

        if filter.is_over_constrained() {
            return Err(anyhow!("only one filter is applicable"));
        }

        if filter.is_empty_filter() {
            // Start filtering
            let documents_with_chunks = documents::Entity::find()
                .find_with_related(document_chunks::Entity)
                .all(&self.pool)
                .await?;

            return Ok(documents_with_chunks
                .into_iter()
                .map(|item| item.into())
                .collect());
        }

        // Construct a sql filter to the table
        let mut sql_filter = sea_orm::Condition::any();

        if !filter.ids.is_empty() {
            sql_filter = sql_filter.add(documents::Column::Id.is_in(&filter.ids));
        }

        if !filter.collection_metadata_ids.is_empty() {
            sql_filter = sql_filter.add(
                documents::Column::CollectionMetadataId.is_in(&filter.collection_metadata_ids),
            );
        }

        if let Some(created_at) = &filter.created_at {
            sql_filter = sql_filter.add(documents::Column::CreatedAt.eq(created_at));
        }

        if let Some(last_modified) = &filter.last_modified {
            sql_filter = sql_filter.add(documents::Column::CreatedAt.eq(last_modified));
        }

        if let Some(title) = &filter.title {
            sql_filter = sql_filter.add(documents::Column::Title.eq(title));
        }

        // Start filtering
        let documents_with_chunks = documents::Entity::find()
            .find_with_related(document_chunks::Entity)
            .filter(sql_filter)
            .all(&self.pool)
            .await?;

        // id ordering only applies to filters with ids
        if !filter.ids.is_empty() {
            Ok(map_order_by_ids(documents_with_chunks, &filter.ids))
        } else {
            Ok(documents_with_chunks
                .into_iter()
                .map(|item| item.into())
                .collect())
        }
    }

    /// It gurantees the order of the metadata will follow the input ids order
    async fn get_collections(
        &self,
        filter: &GetCollectionFilter,
        include_chunk_data: bool,
    ) -> Result<Vec<CollectionMetadata>> {
        use crate::databases::database::entity::{collections, documents};

        if filter.is_over_constrained() {
            return Err(anyhow!("only one filter is applicable"));
        }

        // Construct a sql filter to the table
        let mut sql_filter = sea_orm::Condition::any();

        if !filter.ids.is_empty() {
            sql_filter = sql_filter.add(collections::Column::Id.is_in(&filter.ids));
        }

        if let Some(created_at) = &filter.created_at {
            sql_filter = sql_filter.add(collections::Column::CreatedAt.eq(created_at));
        }

        if let Some(last_modified) = &filter.last_modified {
            sql_filter = sql_filter.add(collections::Column::CreatedAt.eq(last_modified));
        }

        if let Some(title) = &filter.title {
            sql_filter = sql_filter.add(collections::Column::Title.eq(title));
        }

        // Start filtering
        let collections = if filter.is_empty_filter() {
            collections::Entity::find()
                .find_with_related(documents::Entity)
                .all(&self.pool)
                .await?
        } else {
            collections::Entity::find()
                .find_with_related(documents::Entity)
                .filter(sql_filter)
                .all(&self.pool)
                .await?
        };

        // No chunk data yet
        let mut collection_metadatas: Vec<CollectionMetadata> =
            collections.into_iter().map(|item| item.into()).collect();

        // Conduct an additional query if the user decides to include the chunk data
        if !include_chunk_data {
            return Ok(collection_metadatas);
        }

        // Query the chunk details in parallel to reduce round-trips
        let mut tasks = Vec::new();
        for collection in collection_metadatas.iter_mut() {
            tasks.push(async {
                collection.documents_metadatas = self
                    .get_documents(&GetDocumentFilter {
                        collection_metadata_ids: vec![collection.id.clone()],
                        ..Default::default()
                    })
                    .await?;

                Ok::<_, anyhow::Error>(())
            });
        }

        let results = join_all(tasks).await;
        for result in results {
            result?;
        }

        // Id ordering only applies to filtering by ids
        if !filter.ids.is_empty() {
            Ok(map_order_by_ids(collection_metadatas, &filter.ids))
        } else {
            Ok(collection_metadatas)
        }
    }

    async fn peek(&self) -> Result<DatabaseInformation> {
        use crate::databases::database::entity::{collections, documents, users};

        let mut tasks = Vec::new();

        tasks.push(users::Entity::find().count(&self.pool));

        tasks.push(documents::Entity::find().count(&self.pool));

        tasks.push(collections::Entity::find().count(&self.pool));

        let results = join_all(tasks).await;
        let info = DatabaseInformation {
            number_users: results[0].clone()? as usize,
            number_documents: results[1].clone()? as usize,
            number_collections: results[2].clone()? as usize,
        };

        Ok(info)
    }

    async fn search(
        &self,
        query: &str,
        document_metadata_ids: &Vec<String>,
    ) -> Result<Vec<DocumentChunk>> {
        use crate::databases::database::entity::document_chunks;

        let result = document_chunks::Entity::find()
            .filter(document_chunks::Column::DocumentMetadataId.is_in(document_metadata_ids))
            .filter(document_chunks::Column::Content.like(format!("%{}%", query)))
            .all(&self.pool)
            .await?;

        Ok(result.into_iter().map(|item| item.into()).collect())
    }
}
