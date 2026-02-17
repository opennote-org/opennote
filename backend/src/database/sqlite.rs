use std::{str::FromStr, time::Duration};

use actix_web::cookie::time::UtcDateTime;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use sqlx::{
    Row, SqlitePool,
    migrate::{MigrateDatabase, Migrator},
    sqlite::SqliteConnectOptions,
};

use crate::{
    configurations::user::UserConfigurations,
    database::{
        metadata::MetadataSettings,
        traits::{database::Database, identities::Identities, metadata::MetadataManagement},
        utilities::parse_timestamp,
    },
    documents::{
        collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata,
        traits::ValidateDataMutabilitiesForAPICaller,
    },
    identities::user::User,
    metadata_storage::MetadataStorage,
    traits::LoadAndSave,
};

static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"

#[derive(Debug, Clone)]
pub struct SQLiteDatabase {
    pool: SqlitePool,
}

#[async_trait]
impl Database for SQLiteDatabase {
    /// Perform upgrades to the existing database
    /// The `path` is to specify the legacy `metadata_storage` path
    async fn migrate(&self, path: &str) -> Result<()> {
        MIGRATOR.run(&self.pool).await?;

        let metadata_storage = MetadataStorage::load(path)?;
        let mut connection = self.pool.acquire().await?;

        // migrate the global settings in metadata storage first
        sqlx::query(
            "UPDATE metadata_settings
             SET embedder_model_in_use = ?, embedder_model_vector_size_in_use = ?
             WHERE id = 1",
        )
        .bind(&metadata_storage.embedder_model_in_use)
        .bind(metadata_storage.embedder_model_vector_size_in_use as i64)
        .execute(&mut *connection)
        .await?;

        // migrate all collection records
        for (_, collection) in metadata_storage.collections {
            sqlx::query(
                "INSERT OR IGNORE INTO collections (id, title, created_at, last_modified)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&collection.id)
            .bind(&collection.title)
            .bind(parse_timestamp(&collection.created_at))
            .bind(parse_timestamp(&collection.last_modified))
            .execute(&mut *connection)
            .await?;
        }

        // migrate all document records
        for (_, document) in metadata_storage.documents {
            sqlx::query(
                "INSERT OR IGNORE INTO documents (id, collection_metadata_id, title, created_at, last_modified)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&document.id)
            .bind(&document.collection_metadata_id)
            .bind(&document.title)
            .bind(parse_timestamp(&document.created_at))
            .bind(parse_timestamp(&document.last_modified))
            .execute(&mut *connection)
            .await?;

            // migrate all document chunks
            for (chunk_order, document_chunk_id) in document.chunks.iter().enumerate() {
                sqlx::query(
                    "INSERT OR IGNORE INTO document_chunks (id, document_metadata_id, collection_metadata_id, content, chunk_order, dense_text_vector)
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(document_chunk_id)
                .bind(&document.id)
                .bind(&document.collection_metadata_id)
                .bind("") // Content is not available in metadata storage
                .bind(chunk_order as i64)
                .bind(Vec::<u8>::new()) // Dense vector is not available in metadata storage
                .execute(&mut *connection)
                .await?;
            }
        }

        Ok(())
    }
}

impl SQLiteDatabase {
    /// It will load the existing database, otherwise it will create a new one
    pub async fn new(connection_url: &str) -> Result<Self> {
        if !sqlx::Sqlite::database_exists(connection_url).await? {
            sqlx::Sqlite::create_database(connection_url).await?;
            log::info!("Database does not exist. Created a new one");
        }

        let options = SqliteConnectOptions::from_str(connection_url)?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Identities for SQLiteDatabase {
    async fn create_user(&self, username: String, password: String) -> Result<()> {
        let user = User::new(username, password);
        let config_json = serde_json::to_string(&user.configuration)?;

        sqlx::query(
            "INSERT INTO users (id, username, password, configuration) VALUES (?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password)
        .bind(config_json)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create user: {}", e))?;

        Ok(())
    }

    async fn add_users(&self, users: Vec<User>) -> Result<()> {
        if users.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        let mut user_data = Vec::new();

        for user in users.iter() {
            let config_json = serde_json::to_string(&user.configuration).map_err(|e| {
                anyhow!(
                    "Failed to serialize configuration for user {}: {}",
                    user.username,
                    e
                )
            })?;
            user_data.push((&user.id, &user.username, &user.password, config_json));
        }

        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("INSERT INTO users (id, username, password, configuration) ");

        query_builder.push_values(user_data, |mut b, (id, username, password, config)| {
            b.push_bind(id)
                .push_bind(username)
                .push_bind(password)
                .push_bind(config);
        });

        let query = query_builder.build();

        query
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Failed to batch insert users: {}", e))?;

        tx.commit().await?;
        Ok(())
    }

    async fn delete_users(&self, usernames: Vec<String>) -> Result<Vec<User>> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        let mut transaction = self.pool.begin().await?;

        // 1. Fetch all users in one query
        // Construct "SELECT ... WHERE username IN (?, ?, ...)"
        let query_str = format!(
            "SELECT id, username, password, configuration FROM users WHERE username IN ({})",
            vec!["?"; usernames.len()].join(",")
        );

        let mut query = sqlx::query(&query_str);
        for username in &usernames {
            query = query.bind(username);
        }

        let user_rows = query.fetch_all(&mut *transaction).await?;

        // Collect IDs for resource fetching and deletion
        let user_ids: Vec<String> = user_rows.iter().map(|r| r.get("id")).collect();

        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Fetch all resources in one query
        let resource_query_str = format!(
            "SELECT user_id, resource_id FROM user_resources WHERE user_id IN ({})",
            vec!["?"; user_ids.len()].join(",")
        );

        let mut resource_query = sqlx::query(&resource_query_str);
        for id in &user_ids {
            resource_query = resource_query.bind(id);
        }

        let resource_rows = resource_query.fetch_all(&mut *transaction).await?;

        // Group resources by user_id
        let mut resources_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in resource_rows {
            let user_id: String = row.get("user_id");
            let resource_id: String = row.get("resource_id");
            resources_map.entry(user_id).or_default().push(resource_id);
        }

        // 3. Construct the result list
        let mut deleted_users = Vec::with_capacity(user_rows.len());
        for row in user_rows {
            let id: String = row.get("id");
            let username: String = row.get("username");
            let password: String = row.get("password");
            let config_json: String = row.get("configuration");
            // Gracefully handle JSON errors or propagate them
            let configuration: UserConfigurations = serde_json::from_str(&config_json)?;

            let resources = resources_map.remove(&id).unwrap_or_default();

            deleted_users.push(User {
                id,
                username,
                password,
                resources,
                configuration,
            });
        }

        // 4. Delete users in one query (Cascades to user_resources)
        let delete_query_str = format!(
            "DELETE FROM users WHERE id IN ({})",
            vec!["?"; user_ids.len()].join(",")
        );

        let mut delete_query = sqlx::query(&delete_query_str);
        for id in &user_ids {
            delete_query = delete_query.bind(id);
        }

        delete_query.execute(&mut *transaction).await?;

        transaction.commit().await?;

        Ok(deleted_users)
    }

    async fn validate_user_password(&self, username: &str, password: &str) -> Result<bool> {
        let row = sqlx::query("SELECT password FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let stored_password: String = row.get("password");
            return Ok(stored_password == password);
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    async fn add_authorized_resources(
        &self,
        username: &str,
        resource_ids: Vec<String>,
    ) -> Result<()> {
        let user_row = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = user_row {
            let user_id: String = row.get("id");
            for resource_id in resource_ids {
                sqlx::query(
                    "INSERT OR IGNORE INTO user_resources (user_id, resource_id) VALUES (?, ?)",
                )
                .bind(&user_id)
                .bind(resource_id)
                .execute(&self.pool)
                .await?;
            }
            Ok(())
        } else {
            Err(anyhow!("User `{}` does not exist", username))
        }
    }

    async fn remove_authorized_resources(
        &self,
        username: &str,
        resource_ids: Vec<String>,
    ) -> Result<()> {
        let user_row = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = user_row {
            let user_id: String = row.get("id");
            for resource_id in resource_ids {
                sqlx::query("DELETE FROM user_resources WHERE user_id = ? AND resource_id = ?")
                    .bind(&user_id)
                    .bind(resource_id)
                    .execute(&self.pool)
                    .await?;
            }
            Ok(())
        } else {
            Err(anyhow!("User `{}` does not exist", username))
        }
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
        let config_json = serde_json::to_string(&user_configurations)?;

        let result = sqlx::query("UPDATE users SET configuration = ? WHERE username = ?")
            .bind(config_json)
            .bind(username)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            Err(anyhow!("User `{}` does not exist", username))
        } else {
            Ok(())
        }
    }

    async fn get_user_configurations(&self, username: &str) -> Result<UserConfigurations> {
        let row = sqlx::query("SELECT configuration FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let config_json: String = row.get("configuration");
            let config: UserConfigurations = serde_json::from_str(&config_json)?;
            Ok(config)
        } else {
            Err(anyhow!("User `{}` does not exist", username))
        }
    }

    async fn get_users_by_resource_id(&self, id: &str) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT u.id, u.username, u.password, u.configuration
             FROM users u
             JOIN user_resources ur ON u.id = ur.user_id
             WHERE ur.resource_id = ?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_id: String = row.get("id");
            let username: String = row.get("username");
            let password: String = row.get("password");
            let config_json: String = row.get("configuration");
            let configuration: UserConfigurations = serde_json::from_str(&config_json)?;

            let resource_rows =
                sqlx::query("SELECT resource_id FROM user_resources WHERE user_id = ?")
                    .bind(&user_id)
                    .fetch_all(&self.pool)
                    .await?;

            let resources: Vec<String> =
                resource_rows.iter().map(|r| r.get("resource_id")).collect();

            users.push(User {
                id: user_id,
                username,
                password,
                resources,
                configuration,
            });
        }

        Ok(users)
    }

    async fn get_resource_ids_by_username(&self, username: &str) -> Result<Vec<String>> {
        let user_row = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = user_row {
            let user_id: String = row.get("id");
            let resource_rows =
                sqlx::query("SELECT resource_id FROM user_resources WHERE user_id = ?")
                    .bind(&user_id)
                    .fetch_all(&self.pool)
                    .await?;

            Ok(resource_rows.iter().map(|r| r.get("resource_id")).collect())
        } else {
            Err(anyhow!("User `{}` does not exist", username))
        }
    }

    async fn is_user_owning_collections(
        &self,
        username: &str,
        collection_metadata_ids: &[String],
    ) -> Result<bool> {
        let user_row = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = user_row {
            let user_id: String = row.get("id");
            // Check if all collection_metadata_ids are in user_resources
            for id in collection_metadata_ids {
                let exists = sqlx::query(
                    "SELECT 1 FROM user_resources WHERE user_id = ? AND resource_id = ?",
                )
                .bind(&user_id)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
                if exists.is_none() {
                    return Ok(false);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query("SELECT id, username, password, configuration FROM users")
            .fetch_all(&self.pool)
            .await?;

        let mut users = Vec::with_capacity(rows.len());

        for row in rows {
            let id: String = row.get("id");
            let username: String = row.get("username");
            let password: String = row.get("password");
            let config_json: String = row.get("configuration");
            let configuration: UserConfigurations = serde_json::from_str(&config_json)?;

            // Fetch resource IDs for this user
            let resource_rows =
                sqlx::query("SELECT resource_id FROM user_resources WHERE user_id = ?")
                    .bind(&id)
                    .fetch_all(&self.pool)
                    .await?;

            let resources: Vec<String> =
                resource_rows.iter().map(|r| r.get("resource_id")).collect();

            users.push(User {
                id,
                username,
                password,
                resources,
                configuration,
            });
        }

        Ok(users)
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

    async fn delete_collection(
        &self,
        collection_metadata_id: &str,
    ) -> Option<CollectionMetadata> {
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
        let doc_rows =
            sqlx::query("SELECT id, collection_metadata_id FROM documents ORDER BY id ASC")
                .fetch_all(&self.pool)
                .await?;

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
