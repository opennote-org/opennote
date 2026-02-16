pub mod metadata;
pub mod traits;

mod utilities;

use anyhow::Result;
use sqlx::{migrate::{MigrateDatabase, Migrator}, SqlitePool};

use crate::{
    database::utilities::parse_timestamp, metadata_storage::MetadataStorage, traits::LoadAndSave,
};

static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// It will load the existing database, otherwise it will create a new one
    pub async fn new(connection_url: &str) -> Result<Self> {
        if !sqlx::Sqlite::database_exists(connection_url).await? {
            sqlx::Sqlite::create_database(connection_url).await?;
            log::info!("Database does not exist. Created a new one");
        }
        
        let pool: sqlx::Pool<sqlx::Sqlite> = SqlitePool::connect(connection_url).await?;

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
