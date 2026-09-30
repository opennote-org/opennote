use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{
    configurations::{desktop::DesktopConfigurations, system::SystemConfigurations},
    key_mappings::KeyMappingConfigurations,
    metadata::{MetaChangesHandling, Metadata},
    traits::LoadFromAndSaveToFile,
};

use crate::configurations::{get_configuration_folder_path, get_metadata};

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

        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations)),
            key_mappings: Arc::new(Mutex::new(key_mappings)),
            databases,
            embedders,
        })
    }

    pub async fn analyze_changes_handling(&self) -> Result<MetaChangesHandling> {
        let configurations = self.configurations.lock().await;
        let system_configurations = &configurations.system;
        let metadata = get_metadata(crate::configurations::ApplicationType::Desktop)?;

        let changes = metadata.detect_changes(system_configurations);

        // Database is more fundamental.
        // If the database has changed, the vector database needs to be reset.
        if changes.database_changed {
            return Ok(MetaChangesHandling {
                reset_vector_database: true,
                reindex_vector_database: false,
            });
        }

        // We just need to reindex the vector database,
        // if only the vector database and the embedding model have changed,
        // because the vector database is a branch of the database.
        if changes.vector_database_changed || changes.embedding_model_changed {
            return Ok(MetaChangesHandling {
                reset_vector_database: false,
                reindex_vector_database: true,
            });
        }

        return Ok(MetaChangesHandling {
            reset_vector_database: false,
            reindex_vector_database: false,
        });
    }

    /// Handle the cases when reindex is needed
    pub async fn handle_changes(&self, handling: MetaChangesHandling) -> Result<()> {
        let configurations = self.configurations.lock().await;
        let system_configurations = &configurations.system;

        if handling.reset_vector_database {
            self.databases
                .vector_database
                .reset_index(
                    &system_configurations.vector_database.index,
                    system_configurations.embedder.dimensions,
                )
                .await?;

            return Ok(());
        }

        if handling.reindex_vector_database {
            self.databases
                .vector_database
                .reindex_documents(
                    system_configurations,
                    &self.databases.database,
                    &self.embedders,
                )
                .await?;
        }

        Ok(())
    }
}
