//! This file defines the configurations that are set in the configurations file.
//! They are not mutable during the runtime and are loaded when the program starts.
//! Modifications to these may incur break changes to the existing database.

use serde::{Deserialize, Serialize};

use crate::configurations::fields::{
    DatabaseConfig, EmbedderConfig, LoggingConfig, VectorDatabaseConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigurations {
    /// Logging settings
    pub logging: LoggingConfig,

    /// Configure the database
    pub database: DatabaseConfig,

    /// Configure the vector database
    pub vector_database: VectorDatabaseConfig,

    /// Configure the embedder to use
    pub embedder: EmbedderConfig,
}

impl Default for SystemConfigurations {
    fn default() -> Self {
        Self {
            logging: LoggingConfig::default(),
            database: DatabaseConfig::default(),
            vector_database: VectorDatabaseConfig::default(),
            embedder: EmbedderConfig::default(),
        }
    }
}
