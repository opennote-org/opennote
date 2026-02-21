use std::time::Duration;

use crate::traits::LoadAndSave;
use crate::{identities::storage::IdentitiesStorage, metadata_storage::MetadataStorage};
use actix_web::cookie::time::UtcDateTime;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
    sea_query::{Expr, OnConflict},
};

use crate::{
    configurations::user::UserConfigurations,
    database::{
        filters::get_users::GetUserFilter,
        metadata::MetadataSettings,
        traits::{database::Database, identities::Identities, metadata::MetadataManagement},
        utilities::parse_timestamp,
    },
    documents::{
        collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata,
        traits::ValidateDataMutabilitiesForAPICaller,
    },
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
        use crate::database::entity::users;

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
        use crate::database::entity::collections;
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

    async fn migrate_documents(&self, metadata_storage: &MetadataStorage) -> Result<()> {
        // migrate all document records
        use crate::database::entity::document_chunks;
        use crate::database::entity::documents;

        let mut documents_to_insert = Vec::new();
        let mut chunks_to_insert = Vec::new();
        for (_, document) in metadata_storage.documents.iter() {
            documents_to_insert.push(documents::ActiveModel {
                id: Set(document.id.clone()),
                collection_metadata_id: Set(document.collection_metadata_id.clone()),
                title: Set(document.title.to_string()),
                created_at: Set(parse_timestamp(&document.created_at)),
                last_modified: Set(parse_timestamp(&document.last_modified)),
                ..Default::default()
            });

            // migrate all document chunks
            for (chunk_order, document_chunk_id) in document.chunks.iter().enumerate() {
                chunks_to_insert.push(document_chunks::ActiveModel {
                    id: Set(document_chunk_id.clone()),
                    document_metadata_id: Set(document.id.clone()),
                    collection_metadata_id: Set(document.collection_metadata_id.clone()),
                    content: Set("".to_string()),
                    chunk_order: Set(chunk_order as i64),
                    dense_text_vector: Set(Vec::new()),
                    ..Default::default()
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
        use crate::database::entity::metadata_settings;
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
    ) -> Result<()> {
        Migrator::up(&self.pool, None).await?;

        let metadata_storage = MetadataStorage::load(metadata_storage_path)?;
        let identities_storage = IdentitiesStorage::load(identities_storage_path)?;

        self.migrate_metadata_settings(&metadata_storage).await?;
        self.migrate_collections(&metadata_storage).await?;
        self.migrate_documents(&metadata_storage).await?;
        self.migrate_users(&identities_storage).await?;

        Ok(())
    }
}

impl SQLiteDatabase {
    /// It will load the existing database, otherwise it will create a new one
    pub async fn new(connection_url: &str) -> Result<Self> {
        // sea-orm will create file when it does not exist,
        // therefore, we don't need to do a manual check like we did when
        // using sqlx
        let mut options = ConnectOptions::new(connection_url);
        options.map_sqlx_sqlite_opts(|options| {
            options
                .busy_timeout(Duration::from_secs(5))
                .journal_mode(sea_orm::sqlx::sqlite::SqliteJournalMode::Wal)
        });

        let pool = sea_orm::Database::connect(options).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Identities for SQLiteDatabase {
    async fn create_user(&self, username: String, password: String) -> Result<()> {
        use crate::database::entity::users::*;

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

        use crate::database::entity::users::*;

        let users: Vec<ActiveModel> = users.into_iter().map(|item| item.into()).collect();

        Entity::insert_many(users).exec(&self.pool).await?;

        Ok(())
    }

    async fn delete_users(&self, usernames: Vec<String>) -> Result<Vec<User>> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        use crate::database::entity;

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
        );

        Ok(users.into_iter().map(|item| item.into()).collect())
    }

    async fn validate_user_password(&self, username: &str, password: &str) -> Result<bool> {
        use crate::database::entity::users;

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
        use crate::database::entity::users;

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
        resource_ids: Vec<String>,
    ) -> Result<()> {
        use crate::database::entity::users;

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

    async fn check_permission(&self, username: &str, resource_ids: Vec<String>) -> Result<bool> {
        self.is_user_owning_collections(username, &resource_ids)
            .await
    }

    async fn update_user_configurations(
        &self,
        username: &str,
        user_configurations: UserConfigurations,
    ) -> Result<()> {
        use crate::database::entity::users;

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
        use crate::database::entity::users;

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
        use crate::database::entity::users;

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
        use crate::database::entity::users;

        if filter.is_over_constrained() {
            return Err(anyhow!("only one filter is applicable"));
        }

        // Construct a sql filter to the users table
        let mut sql_filter_to_users = sea_orm::Condition::any();

        if let Some(id) = &filter.id {
            sql_filter_to_users = sql_filter_to_users.add(users::Column::Id.eq(id));
        }

        if let Some(username) = &filter.username {
            sql_filter_to_users = sql_filter_to_users.add(users::Column::Username.eq(username));
        }

        if let Some(resource_ids) = &filter.resources {
            let resource_ids =
                serde_json::to_value(&resource_ids).context("Failed to serialize resource ids")?;

            sql_filter_to_users =
                sql_filter_to_users.add(users::Column::ResourceIds.eq(resource_ids));
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
        let collection = CollectionMetadata::new(title.to_string());
        sqlx::query(
            "INSERT INTO collections (id, title, created_at, last_modified) VALUES (?, ?, ?, ?)",
        )
        .bind(&collection.id)
        .bind(&collection.title)
        .bind(parse_timestamp(&collection.created_at))
        .bind(parse_timestamp(&collection.last_modified))
        .execute(&self.pool)
        .await?;
        Ok(collection.id)
    }

    async fn delete_collection(&self, collection_metadata_id: &str) -> Option<CollectionMetadata> {
        let mut tx = self.pool.begin().await.ok()?;

        let row = sqlx::query(
            "SELECT id, title, created_at, last_modified FROM collections WHERE id = ?",
        )
        .bind(collection_metadata_id)
        .fetch_optional(&mut *tx)
        .await
        .ok()??;

        let id: String = row.get("id");
        let title: String = row.get("title");
        let created_at: i64 = row.get("created_at");
        let last_modified: i64 = row.get("last_modified");

        let doc_rows = sqlx::query("SELECT id FROM documents WHERE collection_metadata_id = ?")
            .bind(collection_metadata_id)
            .fetch_all(&mut *tx)
            .await
            .ok()?;

        let documents_metadata_ids: Vec<String> = doc_rows.iter().map(|r| r.get("id")).collect();

        let collection = CollectionMetadata {
            id,
            title,
            created_at: UtcDateTime::from_unix_timestamp(created_at)
                .unwrap()
                .to_string(),
            last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                .unwrap()
                .to_string(),
            documents_metadata_ids,
        };

        sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(collection_metadata_id)
            .execute(&mut *tx)
            .await
            .ok()?;

        tx.commit().await.ok()?;
        Some(collection)
    }

    async fn update_collection(
        &self,
        mut collection_metadatas: Vec<CollectionMetadata>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for metadata in collection_metadatas.iter_mut() {
            let exists = sqlx::query("SELECT 1 FROM collections WHERE id = ?")
                .bind(&metadata.id)
                .fetch_optional(&mut *tx)
                .await?;

            if exists.is_some() {
                metadata.is_mutated()?;

                metadata.last_modified = UtcDateTime::now().to_string();

                sqlx::query("UPDATE collections SET title = ?, last_modified = ? WHERE id = ?")
                    .bind(&metadata.title)
                    .bind(parse_timestamp(&metadata.last_modified))
                    .bind(&metadata.id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                return Err(anyhow!(
                    "Collection metadata id {} was not found, update operation terminated",
                    metadata.id
                ));
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn update_documents_with_new_chunks(
        &self,
        document_metadatas: Vec<DocumentMetadata>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for metadata in document_metadatas {
            let exists = sqlx::query("SELECT 1 FROM documents WHERE id = ?")
                .bind(&metadata.id)
                .fetch_optional(&mut *tx)
                .await?;

            if exists.is_some() {
                // Update basic fields just in case (MetadataStorage replaces entire object)
                sqlx::query("UPDATE documents SET title = ?, last_modified = ?, collection_metadata_id = ? WHERE id = ?")
                    .bind(&metadata.title)
                    .bind(parse_timestamp(&metadata.last_modified))
                    .bind(&metadata.collection_metadata_id)
                    .bind(&metadata.id)
                    .execute(&mut *tx)
                    .await?;

                // Swap the chunks out
                sqlx::query("DELETE FROM document_chunks WHERE document_metadata_id = ?")
                    .bind(&metadata.id)
                    .execute(&mut *tx)
                    .await?;

                for (i, chunk_id) in metadata.chunks.iter().enumerate() {
                    sqlx::query("INSERT INTO document_chunks (id, document_metadata_id, collection_metadata_id, content, dense_text_vector, chunk_order) VALUES (?, ?, ?, ?, ?, ?)")
                        .bind(chunk_id)
                        .bind(&metadata.id)
                        .bind(&metadata.collection_metadata_id)
                        .bind("")
                        .bind(Vec::<u8>::new())
                        .bind(i as i64)
                        .execute(&mut *tx)
                        .await?;
                }
            } else {
                return Err(anyhow!(
                    "Document metadata id {} was not found, update operation terminated",
                    metadata.id
                ));
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn verify_immutable_fields_in_document_metadatas(
        &self,
        document_metadatas: &mut Vec<DocumentMetadata>,
    ) -> Result<()> {
        for metadata in document_metadatas.iter_mut() {
            let row = sqlx::query(
                "SELECT created_at, collection_metadata_id FROM documents WHERE id = ?",
            )
            .bind(&metadata.id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(row) = row {
                metadata.is_mutated()?;

                let chunks_rows = sqlx::query("SELECT id FROM document_chunks WHERE document_metadata_id = ? ORDER BY chunk_order ASC")
                    .bind(&metadata.id)
                    .fetch_all(&self.pool)
                    .await?;
                metadata.chunks = chunks_rows.iter().map(|r| r.get("id")).collect();

                let original_collection_id: String = row.get("collection_metadata_id");

                if metadata.collection_metadata_id != original_collection_id {
                    let exists = sqlx::query("SELECT 1 FROM collections WHERE id = ?")
                        .bind(&metadata.collection_metadata_id)
                        .fetch_optional(&self.pool)
                        .await?;
                    if exists.is_none() {
                        return Err(anyhow!(
                            "Target collection id {} was not found, update operation terminated",
                            metadata.collection_metadata_id
                        ));
                    }
                }
            } else {
                return Err(anyhow!(
                    "Document metadata id {} was not found, update operation terminated",
                    metadata.id
                ));
            }
        }
        Ok(())
    }

    async fn update_documents(&self, document_metadatas: Vec<DocumentMetadata>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for mut metadata in document_metadatas {
            let row = sqlx::query("SELECT collection_metadata_id FROM documents WHERE id = ?")
                .bind(&metadata.id)
                .fetch_optional(&mut *tx)
                .await?;

            if let Some(row) = row {
                let old_collection_id: String = row.get("collection_metadata_id");

                let now_ts = UtcDateTime::now().to_string();
                let now_i64 = parse_timestamp(&now_ts);
                metadata.last_modified = now_ts.clone();

                sqlx::query("UPDATE collections SET last_modified = ? WHERE id = ?")
                    .bind(now_i64)
                    .bind(&old_collection_id)
                    .execute(&mut *tx)
                    .await?;

                if metadata.collection_metadata_id != old_collection_id {
                    sqlx::query("UPDATE collections SET last_modified = ? WHERE id = ?")
                        .bind(now_i64)
                        .bind(&metadata.collection_metadata_id)
                        .execute(&mut *tx)
                        .await?;

                    sqlx::query("UPDATE document_chunks SET collection_metadata_id = ? WHERE document_metadata_id = ?")
                        .bind(&metadata.collection_metadata_id)
                        .bind(&metadata.id)
                        .execute(&mut *tx)
                        .await?;
                }

                sqlx::query("UPDATE documents SET title = ?, last_modified = ?, collection_metadata_id = ? WHERE id = ?")
                    .bind(&metadata.title)
                    .bind(now_i64)
                    .bind(&metadata.collection_metadata_id)
                    .bind(&metadata.id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn add_document(&self, metadata: DocumentMetadata) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let exists = sqlx::query("SELECT 1 FROM collections WHERE id = ?")
            .bind(&metadata.collection_metadata_id)
            .fetch_optional(&mut *tx)
            .await?;

        if exists.is_some() {
            sqlx::query("INSERT INTO documents (id, collection_metadata_id, title, created_at, last_modified) VALUES (?, ?, ?, ?, ?)")
                .bind(&metadata.id)
                .bind(&metadata.collection_metadata_id)
                .bind(&metadata.title)
                .bind(parse_timestamp(&metadata.created_at))
                .bind(parse_timestamp(&metadata.last_modified))
                .execute(&mut *tx)
                .await?;

            for (i, chunk_id) in metadata.chunks.iter().enumerate() {
                sqlx::query("INSERT INTO document_chunks (id, document_metadata_id, collection_metadata_id, content, dense_text_vector, chunk_order) VALUES (?, ?, ?, ?, ?, ?)")
                        .bind(chunk_id)
                        .bind(&metadata.id)
                        .bind(&metadata.collection_metadata_id)
                        .bind("")
                        .bind(Vec::<u8>::new())
                        .bind(i as i64)
                        .execute(&mut *tx)
                        .await?;
            }

            tx.commit().await?;
            Ok(())
        } else {
            Err(anyhow!(
                "Collection {} is missing. Please create a collection before adding new documents to it",
                metadata.collection_metadata_id
            ))
        }
    }

    async fn get_document(&self, docuemnt_metadata_id: &str) -> Option<DocumentMetadata> {
        let row = sqlx::query("SELECT id, title, created_at, last_modified, collection_metadata_id FROM documents WHERE id = ?")
            .bind(docuemnt_metadata_id)
            .fetch_optional(&self.pool)
            .await.ok()??;

        let id: String = row.get("id");
        let title: String = row.get("title");
        let created_at: i64 = row.get("created_at");
        let last_modified: i64 = row.get("last_modified");
        let collection_metadata_id: String = row.get("collection_metadata_id");

        let chunks_rows = sqlx::query("SELECT id FROM document_chunks WHERE document_metadata_id = ? ORDER BY chunk_order ASC")
            .bind(docuemnt_metadata_id)
            .fetch_all(&self.pool)
            .await.ok()?;

        let chunks: Vec<String> = chunks_rows.iter().map(|r| r.get("id")).collect();

        Some(DocumentMetadata {
            id,
            title,
            created_at: UtcDateTime::from_unix_timestamp(created_at)
                .unwrap()
                .to_string(),
            last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                .unwrap()
                .to_string(),
            collection_metadata_id,
            chunks,
        })
    }

    async fn remove_document(&self, metdata_id: &str) -> Option<DocumentMetadata> {
        let doc = self.get_document(metdata_id).await?;

        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(metdata_id)
            .execute(&self.pool)
            .await
            .ok()?;

        Some(doc)
    }

    async fn get_document_ids_by_collection(&self, collection_metadata_id: &str) -> Vec<String> {
        let rows = sqlx::query("SELECT id FROM documents WHERE collection_metadata_id = ?")
            .bind(collection_metadata_id)
            .fetch_all(&self.pool)
            .await
            .ok();

        if let Some(rows) = rows {
            rows.iter().map(|r| r.get("id")).collect()
        } else {
            Vec::new()
        }
    }

    async fn get_all_documents(&self) -> Result<Vec<DocumentMetadata>> {
        let rows = sqlx::query(
            "SELECT id, title, created_at, last_modified, collection_metadata_id FROM documents",
        )
        .fetch_all(&self.pool)
        .await?;

        // Optimization: Fetch all chunk IDs in one go to avoid N+1 query problem
        let chunks_rows = sqlx::query(
            "SELECT id, document_metadata_id FROM document_chunks ORDER BY chunk_order ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut chunks_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in chunks_rows {
            let doc_id: String = row.get("document_metadata_id");
            let chunk_id: String = row.get("id");
            chunks_map.entry(doc_id).or_default().push(chunk_id);
        }

        let mut documents = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let created_at: i64 = row.get("created_at");
            let last_modified: i64 = row.get("last_modified");
            let collection_metadata_id: String = row.get("collection_metadata_id");

            let chunks = chunks_map.remove(&id).unwrap_or_default();

            documents.push(DocumentMetadata {
                id,
                title,
                created_at: UtcDateTime::from_unix_timestamp(created_at)
                    .unwrap()
                    .to_string(),
                last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                    .unwrap()
                    .to_string(),
                collection_metadata_id,
                chunks,
            });
        }

        Ok(documents)
    }

    async fn get_all_collections(&self) -> Result<Vec<CollectionMetadata>> {
        let rows = sqlx::query("SELECT id, title, created_at, last_modified FROM collections")
            .fetch_all(&self.pool)
            .await?;

        // Optimization: Fetch all document IDs in one go to avoid N+1 query problem
        let document_metadatas = self.get_all_documents().await?;

        let mut docs_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in document_metadatas {
            docs_map
                .entry(row.collection_metadata_id)
                .or_default()
                .push(row.id);
        }

        let mut collections = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let created_at: i64 = row.get("created_at");
            let last_modified: i64 = row.get("last_modified");

            let documents_metadata_ids = docs_map.remove(&id).unwrap_or_default();

            collections.push(CollectionMetadata {
                id,
                title,
                created_at: UtcDateTime::from_unix_timestamp(created_at)
                    .unwrap()
                    .to_string(),
                last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                    .unwrap()
                    .to_string(),
                documents_metadata_ids,
            });
        }

        Ok(collections)
    }

    async fn get_collections_by_collection_metadata_id(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<CollectionMetadata>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build the IN clause placeholders
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let in_clause = placeholders.join(",");

        let query_str = format!(
            "SELECT id, title, created_at, last_modified FROM collections WHERE id IN ({})",
            in_clause
        );

        let mut query = sqlx::query(&query_str);
        for id in &ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(&self.pool).await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Optimization: Fetch all document IDs for these collections in one go
        let found_ids: Vec<String> = rows.iter().map(|r| r.get("id")).collect();
        let doc_placeholders: Vec<&str> = found_ids.iter().map(|_| "?").collect();
        let doc_in_clause = doc_placeholders.join(",");

        let doc_query_str = format!(
            "SELECT id, collection_metadata_id FROM documents WHERE collection_metadata_id IN ({})",
            doc_in_clause
        );

        let mut doc_query = sqlx::query(&doc_query_str);
        for id in &found_ids {
            doc_query = doc_query.bind(id);
        }

        let doc_rows = doc_query.fetch_all(&self.pool).await?;

        let mut docs_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in doc_rows {
            let collection_id: String = row.get("collection_metadata_id");
            let doc_id: String = row.get("id");
            docs_map.entry(collection_id).or_default().push(doc_id);
        }

        let mut collections = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let created_at: i64 = row.get("created_at");
            let last_modified: i64 = row.get("last_modified");

            let documents_metadata_ids = docs_map.remove(&id).unwrap_or_default();

            collections.push(CollectionMetadata {
                id,
                title,
                created_at: UtcDateTime::from_unix_timestamp(created_at)
                    .unwrap()
                    .to_string(),
                last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                    .unwrap()
                    .to_string(),
                documents_metadata_ids,
            });
        }

        Ok(collections)
    }

    async fn get_metadata_settings(&self) -> Result<MetadataSettings> {
        let row = sqlx::query("SELECT embedder_model_in_use, embedder_model_vector_size_in_use FROM metadata_settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(MetadataSettings {
            embedder_model_in_use: row.get("embedder_model_in_use"),
            embedder_model_vector_size_in_use: row
                .get::<i64, _>("embedder_model_vector_size_in_use")
                as usize,
        })
    }

    async fn update_metadata_settings(
        &self,
        metadata_settings: MetadataSettings,
    ) -> Result<MetadataSettings> {
        sqlx::query("UPDATE metadata_settings SET embedder_model_in_use = ?, embedder_model_vector_size_in_use = ? WHERE id = 1")
            .bind(&metadata_settings.embedder_model_in_use)
            .bind(metadata_settings.embedder_model_vector_size_in_use as i64)
            .execute(&self.pool)
            .await?;
        Ok(metadata_settings)
    }

    async fn delete_collections(
        &self,
        collection_metadata_ids: &Vec<String>,
    ) -> Result<Vec<CollectionMetadata>> {
        if collection_metadata_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;

        // Build IN clause for collections
        let placeholders: Vec<&str> = collection_metadata_ids.iter().map(|_| "?").collect();
        let in_clause = placeholders.join(",");

        // Fetch full CollectionMetadata before deletion
        let query_str = format!(
            "SELECT id, title, created_at, last_modified FROM collections WHERE id IN ({})",
            in_clause
        );
        let mut query = sqlx::query(&query_str);
        for id in collection_metadata_ids {
            query = query.bind(id);
        }
        let rows = query.fetch_all(&mut *tx).await?;

        // Build map of doc_ids per collection for metadata
        // We need this to return the complete metadata of deleted collections
        let doc_query_str = format!(
            "SELECT id, collection_metadata_id FROM documents WHERE collection_metadata_id IN ({})",
            in_clause
        );
        let mut doc_query = sqlx::query(&doc_query_str);
        for id in collection_metadata_ids {
            doc_query = doc_query.bind(id);
        }
        let doc_rows = doc_query.fetch_all(&mut *tx).await?;

        let mut docs_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in doc_rows {
            let collection_id: String = row.get("collection_metadata_id");
            let doc_id: String = row.get("id");
            docs_map.entry(collection_id).or_default().push(doc_id);
        }

        let mut deleted = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let created_at: i64 = row.get("created_at");
            let last_modified: i64 = row.get("last_modified");

            let documents_metadata_ids = docs_map.remove(&id).unwrap_or_default();

            deleted.push(CollectionMetadata {
                id,
                title,
                created_at: UtcDateTime::from_unix_timestamp(created_at)
                    .unwrap_or_else(|_| UtcDateTime::from_unix_timestamp(0).unwrap())
                    .to_string(),
                last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                    .unwrap_or_else(|_| UtcDateTime::from_unix_timestamp(0).unwrap())
                    .to_string(),
                documents_metadata_ids,
            });
        }

        // Batch delete collections
        // With ON DELETE CASCADE enabled in the schema, this will automatically delete
        // dependent documents and document_chunks.
        let delete_query_str = format!("DELETE FROM collections WHERE id IN ({})", in_clause);
        let mut delete_query = sqlx::query(&delete_query_str);
        for id in collection_metadata_ids {
            delete_query = delete_query.bind(id);
        }
        delete_query.execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(deleted)
    }

    async fn delete_documents(
        &self,
        document_metadata_ids: &Vec<String>,
    ) -> Result<Vec<DocumentMetadata>> {
        if document_metadata_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;

        // Build IN clause for documents
        let placeholders: Vec<&str> = document_metadata_ids.iter().map(|_| "?").collect();
        let in_clause = placeholders.join(",");

        // Fetch full DocumentMetadata before deletion
        let query_str = format!(
            "SELECT id, title, created_at, last_modified, collection_metadata_id FROM documents WHERE id IN ({})",
            in_clause
        );
        let mut query = sqlx::query(&query_str);
        for id in document_metadata_ids {
            query = query.bind(id);
        }
        let rows = query.fetch_all(&mut *tx).await?;

        // Build map of chunk_ids per document for metadata
        let chunk_query_str = format!(
            "SELECT id, document_metadata_id FROM document_chunks WHERE document_metadata_id IN ({}) ORDER BY chunk_order ASC",
            in_clause
        );
        let mut chunk_query = sqlx::query(&chunk_query_str);
        for id in document_metadata_ids {
            chunk_query = chunk_query.bind(id);
        }
        let chunk_rows = chunk_query.fetch_all(&mut *tx).await?;

        let mut chunks_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in chunk_rows {
            let doc_id: String = row.get("document_metadata_id");
            let chunk_id: String = row.get("id");
            chunks_map.entry(doc_id).or_default().push(chunk_id);
        }

        let mut deleted = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let created_at: i64 = row.get("created_at");
            let last_modified: i64 = row.get("last_modified");
            let collection_metadata_id: String = row.get("collection_metadata_id");

            let chunks = chunks_map.remove(&id).unwrap_or_default();

            deleted.push(DocumentMetadata {
                id,
                title,
                created_at: UtcDateTime::from_unix_timestamp(created_at)
                    .unwrap_or_else(|_| UtcDateTime::from_unix_timestamp(0).unwrap())
                    .to_string(),
                last_modified: UtcDateTime::from_unix_timestamp(last_modified)
                    .unwrap_or_else(|_| UtcDateTime::from_unix_timestamp(0).unwrap())
                    .to_string(),
                collection_metadata_id,
                chunks,
            });
        }

        // Batch delete documents
        // With ON DELETE CASCADE enabled in the schema, this will automatically delete
        // dependent document_chunks.
        let delete_query_str = format!("DELETE FROM documents WHERE id IN ({})", in_clause);
        let mut delete_query = sqlx::query(&delete_query_str);
        for id in document_metadata_ids {
            delete_query = delete_query.bind(id);
        }
        delete_query.execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(deleted)
    }

    async fn add_collections(&self, collection_metadatas: Vec<CollectionMetadata>) -> Result<()> {
        if collection_metadatas.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
            "INSERT INTO collections (id, title, created_at, last_modified) ",
        );

        query_builder.push_values(collection_metadatas, |mut b, collection| {
            b.push_bind(collection.id)
                .push_bind(collection.title)
                .push_bind(parse_timestamp(&collection.created_at))
                .push_bind(parse_timestamp(&collection.last_modified));
        });

        query_builder.build().execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn add_documents(&self, document_metadatas: Vec<DocumentMetadata>) -> Result<()> {
        if document_metadatas.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Verify collections exist
        let collection_ids: Vec<&String> = document_metadatas
            .iter()
            .map(|d| &d.collection_metadata_id)
            .collect();

        if !collection_ids.is_empty() {
            let placeholders: Vec<&str> = collection_ids.iter().map(|_| "?").collect();
            let query = format!(
                "SELECT id FROM collections WHERE id IN ({})",
                placeholders.join(",")
            );

            let mut q = sqlx::query(&query);
            for id in &collection_ids {
                q = q.bind(id);
            }

            let rows = q.fetch_all(&mut *tx).await?;
            let existing_ids: std::collections::HashSet<String> =
                rows.into_iter().map(|r| r.get("id")).collect();

            for id in collection_ids {
                if !existing_ids.contains(id) {
                    return Err(anyhow!(
                        "Collection {} is missing. Please create a collection before adding new documents to it",
                        id
                    ));
                }
            }
        }

        // Insert documents
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
            "INSERT INTO documents (id, collection_metadata_id, title, created_at, last_modified) ",
        );

        query_builder.push_values(&document_metadatas, |mut b, metadata| {
            b.push_bind(metadata.id.clone())
                .push_bind(metadata.collection_metadata_id.clone())
                .push_bind(metadata.title.clone())
                .push_bind(parse_timestamp(&metadata.created_at))
                .push_bind(parse_timestamp(&metadata.last_modified));
        });

        query_builder.build().execute(&mut *tx).await?;

        // Insert chunks
        let mut all_chunks = Vec::new();
        for metadata in &document_metadatas {
            for (i, chunk_id) in metadata.chunks.iter().enumerate() {
                all_chunks.push((
                    chunk_id,
                    &metadata.id,
                    &metadata.collection_metadata_id,
                    i as i64,
                ));
            }
        }

        for chunk_batch in all_chunks.chunks(50) {
            let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
                "INSERT INTO document_chunks (id, document_metadata_id, collection_metadata_id, content, dense_text_vector, chunk_order) ",
            );

            query_builder.push_values(chunk_batch, |mut b, (chunk_id, doc_id, col_id, order)| {
                b.push_bind(chunk_id)
                    .push_bind(doc_id)
                    .push_bind(col_id)
                    .push_bind("")
                    .push_bind(Vec::<u8>::new())
                    .push_bind(order);
            });

            query_builder.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
