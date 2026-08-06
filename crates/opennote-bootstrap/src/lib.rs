use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{
    configurations::{desktop::DesktopConfigurations, server::ServerConfigurations},
    key_mappings::KeyMappingConfigurations,
};

#[derive(Clone)]
pub struct DesktopBootstrap {
    pub configurations: Arc<Mutex<DesktopConfigurations>>,
    pub key_mappings: Arc<Mutex<KeyMappingConfigurations>>,
    pub databases: Databases,
    pub embedders: EmbedderEntry,
}

impl DesktopBootstrap {
    pub async fn new(
        configurations: &DesktopConfigurations,
        key_mappings: &KeyMappingConfigurations,
    ) -> Result<Self> {
        let embedders = match EmbedderEntry::new(&configurations.system).await {
            Ok(result) => result,
            Err(error) => return Err(anyhow!("Error when loading an embedding model: {}", error)),
        };

        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations.clone())),
            key_mappings: Arc::new(Mutex::new(key_mappings.clone())),
            databases: Databases::new(&configurations.system).await?,
            embedders,
        })
    }
}

#[derive(Clone)]
pub struct ServerBootstrap {
    pub configurations: Arc<Mutex<ServerConfigurations>>,
    pub databases: Databases,
}

// TODO: Separate bootstraps for server and desktop
impl ServerBootstrap {
    pub async fn new(configurations: &ServerConfigurations) -> Result<Self> {
        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations.clone())),
            databases: Databases::new(&configurations.system).await?,
        })
    }
}
