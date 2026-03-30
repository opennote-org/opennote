//! This file defines the configurations that are set in the configurations file.
//! They are not mutable during the runtime and are loaded when the program starts.
//! Modifications to these may incur break changes to the existing database.

use serde::{Deserialize, Serialize};

use crate::{
    constants::{APP_DATA_FOLDER_NAME, DATA_STORAGE_FOLDER_NAME},
    providers::{
        database::DatabaseProvider, embedder::EmbedderProvider,
        vector_database::VectorDatabaseProvider,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigurations {
    /// Server related settings. It will be ignored when using in apps
    pub server: ServerConfig,

    /// Logging settings
    pub logging: LoggingConfig,

    /// Configure the database
    pub database: DatabaseConfig,

    /// Configure the vector database
    pub vector_database: VectorDatabaseConfig,

    /// Configure the embedder to use
    pub embedder: EmbedderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderConfig {
    /// Provider of the embedding model
    pub provider: EmbedderProvider,

    /// base url of your local embedder service.
    pub base_url: String,

    /// Model name of the embedding model
    pub model: String,

    /// Larger number will make the vectorization faster,
    /// but try reducing the number to prevent overflowing the API
    pub vectorization_batch_size: usize,

    /// Dimension of the embedding model
    pub dimensions: usize,

    /// Usually this is a float
    pub encoding_format: String,

    /// API key of the model
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub provider: DatabaseProvider,
    pub connection_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDatabaseConfig {
    pub provider: VectorDatabaseProvider,
    pub index: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LoggingLevel,
    pub format: LoggingFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggingFormat {
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggingLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LoggingLevel::Info,
            format: LoggingFormat::Json,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            workers: 4,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            // Looks like this but should be an absolute path:
            // sqlite://./data/database.sqlite?mode=rwc
            let path_to_sqlite = config_dir
                .join(APP_DATA_FOLDER_NAME)
                .join(DATA_STORAGE_FOLDER_NAME)
                .join("database.sqlite")
                .to_string_lossy()
                .to_string();

            return Self {
                provider: DatabaseProvider::SQLite,
                connection_url: format!("sqlite://{}?mode=rwc", path_to_sqlite),
            };
        }

        panic!("No config directory was found in this system");
    }
}

impl Default for VectorDatabaseConfig {
    fn default() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            // Looks like this but should be an absolute path:
            // ./data
            let vector_database_path = config_dir
                .join(APP_DATA_FOLDER_NAME)
                .join(DATA_STORAGE_FOLDER_NAME)
                .to_string_lossy()
                .to_string();

            return Self {
                provider: VectorDatabaseProvider::Local,
                index: "opennote".to_string(),
                base_url: vector_database_path,
                api_key: "".to_string(),
            };
        }

        panic!("No config directory was found in this system");
    }
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            provider: EmbedderProvider::Native,
            base_url: "".to_string(),
            model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            vectorization_batch_size: 100, // How many vectorization tasks at a time
            dimensions: 384, // sentence-transformers/all-MiniLM-L6-v2 is a 1024 dimensional model
            encoding_format: "float".to_string(),
            api_key: "".to_string(),
        }
    }
}

impl Default for SystemConfigurations {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            database: DatabaseConfig::default(),
            vector_database: VectorDatabaseConfig::default(),
            embedder: EmbedderConfig::default(),
        }
    }
}
