use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{
    configurations::{desktop::DesktopConfigurations, server::ServerConfigurations},
    key_mappings::KeyMappings,
};

#[derive(Clone)]
pub struct DesktopBootstrap {
    pub configurations: Arc<Mutex<DesktopConfigurations>>,
    pub key_mappings: Arc<Mutex<KeyMappings>>,
    pub databases: Databases,
    pub embedders: Option<EmbedderEntry>,
}

// TODO: Separate bootstraps for server and desktop
impl DesktopBootstrap {
    pub async fn new(
        configurations: &DesktopConfigurations,
        key_mappings: &KeyMappings,
    ) -> Result<Self> {
        let embedders = match EmbedderEntry::new(&configurations.system).await {
            Ok(result) => Some(result),
            Err(error) => {
                log::warn!("Error when loading an embedding model: {}", error);
                None
            }
        };

        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations.clone())),
            key_mappings: Arc::new(Mutex::new(key_mappings.clone())),
            databases: Databases::new(&configurations.system).await?,
            embedders,
        })
    }

    /// Reload an embedder model during  the runtime based on the lastest system configurations
    pub async fn reload_embedder(&mut self) -> Result<()> {
        let system = &self.configurations.lock().await.system;

        self.embedders = Some(EmbedderEntry::new(system).await?);

        Ok(())
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
