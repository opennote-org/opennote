use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{
    configurations::desktop::DesktopConfigurations, key_mappings::KeyMappingConfigurations,
    metadata::Metadata, traits::LoadFromAndSaveToFile,
};

use super::change_handler::handle_changes;

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

    /// Detect if a reindex is needed
    pub async fn is_reindex_needed(&self) -> bool {
        let config_path = get_configuration_folder_path(ApplicationType::Desktop);
        
        let metadata = Metadata::load_from_file(&config_path)
            .context("Failed to load metadata on application start")?;

        let system_configurations = &self.configurations.lock().await.system;

        let changes = metadata.detect_changes(system_configurations);

        changes.embedding_model_changed
    }
}
