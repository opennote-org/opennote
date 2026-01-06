//! This file defines the configurations that are set in the configurations file.
//! They are not mutable during the runtime and are loaded when the program starts. 
//! Modifications to these may incur break changes to the existing database. 

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub metadata_storage: MetadataStorageConfig,
    pub user_information_storage: UserInformationStorageConfig,
    pub database: DatabaseConfig,
    pub embedder: EmbedderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInformationStorageConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataStorageConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderConfig {
    pub base_url: String,
    pub model: String,
    pub vectorization_batch_size: usize,
    pub dimensions: usize,
    pub encoding_format: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub index: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content: String = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Config = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        log::info!("Configuration loaded from: {}", path);
        Ok(config)
    }

    /// Reserved for future uses
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path))?;

        log::info!("Configuration saved to: {}", path);
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        if !["trace", "debug", "info", "warn", "error"].contains(&self.logging.level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid logging level: {}",
                self.logging.level
            ));
        }

        Ok(())
    }
}