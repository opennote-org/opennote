use serde::{Deserialize, Serialize};

use crate::{
    configurations::{
        fields::{DatabaseConfig, EmbedderConfig, VectorDatabaseConfig},
        system::SystemConfigurations,
    },
    constants::METADATA_FILENAME,
    traits::{CompareNecessaryChanges, LoadFromAndSaveToFile},
};

#[derive(Debug)]
pub struct MetadataChanges {
    pub vector_database_changed: bool,
    pub database_changed: bool,
    pub embedding_model_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub last_used_vector_database_configuration: Option<VectorDatabaseConfig>,
    pub last_used_database_configuration: Option<DatabaseConfig>,
    pub last_used_embedder_configuration: Option<EmbedderConfig>,
}

impl Metadata {
    pub fn new(system_configurations: &SystemConfigurations) -> Self {
        Self {
            last_used_vector_database_configuration: Some(
                system_configurations.vector_database.clone(),
            ),
            last_used_database_configuration: Some(system_configurations.database.clone()),
            last_used_embedder_configuration: Some(system_configurations.embedder.clone()),
        }
    }

    /// Detect if any one of the providers had changed
    pub fn detect_changes(&self, system_configurations: &SystemConfigurations) -> MetadataChanges {
        // If the metadata is completely missing,
        // we will opt for the safest route, which is to not rebuild the index.
        if self.last_used_database_configuration.is_none()
            && self.last_used_vector_database_configuration.is_none()
            && self.last_used_embedder_configuration.is_none()
        {
            return MetadataChanges::default();
        }

        let embedding_model_changed = match self.last_used_embedder_configuration {
            Some(ref configuration) => {
                configuration.compare_necessary_changes(&system_configurations.embedder)
            }
            None => true,
        };

        let vector_database_changed = match self.last_used_vector_database_configuration {
            Some(ref configuration) => {
                configuration.compare_necessary_changes(&system_configurations.vector_database)
            }
            None => true,
        };

        MetadataChanges {
            database_changed: self.last_used_database_configuration.as_ref()
                != Some(&system_configurations.database),
            vector_database_changed,
            embedding_model_changed,
        }
    }

    pub fn update(&mut self, configurations: &SystemConfigurations) {
        self.last_used_database_configuration = Some(configurations.database.clone());
        self.last_used_vector_database_configuration = Some(configurations.vector_database.clone());
        self.last_used_embedder_configuration = Some(configurations.embedder.clone());
    }
}

impl Default for MetadataChanges {
    fn default() -> Self {
        Self {
            vector_database_changed: false,
            database_changed: false,
            embedding_model_changed: false,
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            last_used_database_configuration: None,
            last_used_embedder_configuration: None,
            last_used_vector_database_configuration: None,
        }
    }
}

impl LoadFromAndSaveToFile for Metadata {
    fn get_configuration_filename() -> &'static str {
        METADATA_FILENAME
    }
}
