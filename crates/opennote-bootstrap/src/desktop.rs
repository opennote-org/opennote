use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{
    configurations::desktop::DesktopConfigurations, key_mappings::KeyMappingConfigurations,
    metadata::Metadata,
};

use crate::change_handler::handle_changes;

#[derive(Clone)]
pub struct DesktopBootstrap {
    pub configurations: Arc<Mutex<DesktopConfigurations>>,
    pub key_mappings: Arc<Mutex<KeyMappingConfigurations>>,
    pub databases: Databases,
    pub embedders: EmbedderEntry,
}

impl DesktopBootstrap {
    pub async fn new(
        configurations: DesktopConfigurations,
        key_mappings: KeyMappingConfigurations,
        metadata: &Metadata,
    ) -> Result<Self> {
        let embedders = EmbedderEntry::new(&configurations.system)
            .await
            .context("Error when loading an embedding model")?;

        let databases = Databases::new(&configurations.system).await?;

        handle_changes(&configurations.system, &databases, &embedders, metadata).await?;

        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations)),
            key_mappings: Arc::new(Mutex::new(key_mappings)),
            databases,
            embedders,
        })
    }
}
