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
    database::{
        metadata::MetadataSettings, traits::MetadataManagement, utilities::parse_timestamp,
    },
    documents::{
        collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata,
        traits::ValidateDataMutabilitiesForAPICaller,
    },
    metadata_storage::MetadataStorage,
    traits::LoadAndSave,
};

static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"

#[derive(Debug, Clone)]
pub struct SQLiteDatabase {
    pool: SqlitePool,
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

    /// Perform upgrades to the existing database
    /// The `path` is to specify the legacy `metadata_storage` path
    pub async fn migrate(&self, path: &str) -> Result<()> {
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

#[async_trait]
impl MetadataManagement for SQLiteDatabase {
    async fn create_collection(&mut self, title: &str) -> Result<String> {
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
        &mut self,
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
        &mut self,
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
        &mut self,
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

    async fn update_documents(&mut self, document_metadatas: Vec<DocumentMetadata>) -> Result<()> {
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

    async fn add_document(&mut self, metadata: DocumentMetadata) -> Result<()> {
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

    async fn remove_document(&mut self, metdata_id: &str) -> Option<DocumentMetadata> {
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

    async fn get_number_documents(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM documents")
            .fetch_one(&self.pool)
            .await?;
        let count: i64 = row.get("count");
        Ok(count as usize)
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
}
