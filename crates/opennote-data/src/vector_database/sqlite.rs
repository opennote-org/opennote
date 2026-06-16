//! TODO: Vector database should only store vectors and an id. The
//! whole payload should be retrieved from the database

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use rusqlite::Connection;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::search::models::RawSearchResult;
use crate::vector_database::models::IndexPayload;
use crate::{
    search::{keyword::KeywordSearch, semantic::SemanticSearch},
    vector_database::traits::VectorDatabase,
};
use opennote_models::{
    configurations::system::{SystemConfigurations, VectorDatabaseConfig},
    payload::Payload,
};

const SQLITE_VECTOR_MACOS_ARM: &[u8] =
    include_bytes!("../../../../assets/sqlite_extensions/sqlite_vector_macos_arm64.dylib");
const SQLITE_VECTOR_MACOS_X86: &[u8] =
    include_bytes!("../../../../assets/sqlite_extensions/sqlite_vector_macos_x86.dylib");
const SQLITE_VECTOR_LINUX: &[u8] =
    include_bytes!("../../../../assets/sqlite_extensions/sqlite_vector_linux.so");
const SQLITE_VECTOR_WINDOWS: &[u8] =
    include_bytes!("../../../../assets/sqlite_extensions/sqlite_vector_windows.dll");

pub struct SQLiteVectorDatabase {
    index: String,
    connection: Mutex<Connection>,
}

#[async_trait]
impl VectorDatabase for SQLiteVectorDatabase {
    async fn create_index(&self, index: &str, dimensions: usize) -> Result<()> {
        let create_table_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{index}\" (
                id INTEGER PRIMARY KEY,
                payload_id TEXT NOT NULL,
                block_id TEXT NOT NULL,
                vector BLOB
            )",
        );

        let connection = self.connection.lock().await;

        connection
            .execute(&create_table_sql, [])
            .context(format!("Failed to create SQLite vector table '{index}'"))?;

        match connection.query_row(
            &format!(
                "SELECT vector_init('{index}', 'vector', 'type=FLOAT32,dimension={dimensions}')"
            ),
            (),
            |_row| Ok(()),
        ) {
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    async fn validate_data_integrity(
        &self,
        _vector_database_config: &VectorDatabaseConfig,
    ) -> Result<bool> {
        Ok(true)
    }

    async fn create_entries(&self, index: &str, payloads: Vec<Payload>) -> Result<()> {
        let connection = self.connection.lock().await;

        let mut stmt = connection.prepare(&format!(
            "INSERT INTO {}(payload_id, block_id, vector) VALUES(?1, ?2, vector_as_f32(?3))",
            index
        ))?;

        for item in payloads {
            let item: IndexPayload = item.into();

            let vector_str = format!(
                "[{}]",
                item.vector
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            match stmt.execute(rusqlite::params![
                item.payload_id,
                item.block_id,
                vector_str
            ]) {
                Ok(_) => {}
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    async fn delete_index(&self, index: &str) -> Result<()> {
        let sql = format!("DROP TABLE IF EXISTS {}", index);

        let connection = self.connection.lock().await;
        connection.execute(&sql, ())?;

        Ok(())
    }

    async fn delete_entries(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        payload_ids: &Vec<Uuid>,
    ) -> Result<()> {
        if payload_ids.is_empty() {
            return Ok(());
        }

        let placeholders = payload_ids
            .iter()
            .map(|item| format!("'{}'", item.to_string()))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "DELETE FROM {} WHERE payload_id IN ({})",
            vector_database_config.index, placeholders
        );

        let connection = self.connection.lock().await;
        connection.execute(&sql, [])?;

        Ok(())
    }
}

#[async_trait]
impl SemanticSearch for SQLiteVectorDatabase {
    async fn search_documents_semantically(
        &self,
        payload_ids: &Vec<Uuid>,
        query: &[f32],
        top_n: usize,
    ) -> Result<Vec<RawSearchResult>> {
        // Embed the query
        let vector_str = format!(
            "[{}]",
            query
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let connection = self.connection.lock().await;

        // Build IN filter for payload_ids
        let placeholders = payload_ids
            .iter()
            .map(|item| format!("'{}'", item.to_string()))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT payload.payload_id, payload.block_id, search_result.distance
             FROM vector_full_scan(?1, 'vector', vector_as_f32(?2)) AS search_result
             JOIN {} payload ON payload.id = search_result.id
             WHERE payload.payload_id IN ({})
             ORDER BY search_result.distance ASC
             LIMIT ?3",
            self.index, placeholders
        );

        let mut stmt = connection.prepare(&sql)?;

        let results: Vec<RawSearchResult> = stmt
            .query_map(
                rusqlite::params![self.index.clone(), vector_str, top_n as i64],
                |row| {
                    let payload_id: String = row.get(0)?;
                    let block_id: String = row.get(1)?;
                    let score: f32 = row.get(2)?;
                    Ok((payload_id, block_id, score))
                },
            )?
            .filter_map(|r| {
                let (payload_id, block_id, score) = r.ok()?;
                Some(RawSearchResult {
                    payload_id: Uuid::parse_str(&payload_id).ok()?,
                    block_id: Uuid::parse_str(&block_id).ok()?,
                    score,
                })
            })
            .collect();

        Ok(results)
    }
}

#[async_trait]
impl KeywordSearch for SQLiteVectorDatabase {}

impl SQLiteVectorDatabase {
    pub async fn new(configuration: &SystemConfigurations) -> Result<Self> {
        let connection = Connection::open(&configuration.vector_database.base_url)?;

        unsafe {
            let _guard = rusqlite::LoadExtensionGuard::new(&connection).unwrap();
            connection
                .load_extension(load_sqlite_vector_extension()?, None::<&str>)
                .unwrap();
        };

        let vector_database = Self {
            index: configuration.vector_database.index.clone(),
            connection: Mutex::new(connection),
        };

        vector_database
            .create_index(
                &configuration.vector_database.index,
                configuration.embedder.dimensions,
            )
            .await?;

        Ok(vector_database)
    }
}

fn load_sqlite_vector_extension() -> Result<PathBuf> {
    let dir = std::env::temp_dir();
    std::fs::create_dir_all(&dir)?;

    let (bytes, file_extension) = if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        (SQLITE_VECTOR_MACOS_X86, ".dylib")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        (SQLITE_VECTOR_MACOS_ARM, ".dylib")
    } else if cfg!(target_os = "linux") {
        (SQLITE_VECTOR_LINUX, ".so")
    } else if cfg!(target_os = "windows") {
        (SQLITE_VECTOR_WINDOWS, ".dll")
    } else {
        return Err(anyhow!("Unsupported platform"));
    };

    let mut path = dir.join("vector");
    path.set_extension(file_extension.trim_start_matches('.'));

    // Write the extension to a temp file for loading
    std::fs::write(&path, bytes)?;

    Ok(path)
}
