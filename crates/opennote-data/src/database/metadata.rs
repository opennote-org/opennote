use sqlx::prelude::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct MetadataSettings {
    pub vector_database_in_use: String,
    pub embedder_model_in_use: String,
    pub embedder_model_vector_size_in_use: usize,
}

impl From<opennote_entities::metadata_settings::Model> for MetadataSettings {
    fn from(value: opennote_entities::metadata_settings::Model) -> Self {
        Self {
            vector_database_in_use: value.vector_database_in_use,
            embedder_model_in_use: value.embedder_model_in_use,
            embedder_model_vector_size_in_use: value.embedder_model_vector_size_in_use as usize,
        }
    }
}

impl From<MetadataSettings> for opennote_entities::metadata_settings::Model {
    fn from(value: MetadataSettings) -> Self {
        Self {
            id: 1,
            vector_database_in_use: value.vector_database_in_use,
            embedder_model_in_use: value.embedder_model_in_use,
            embedder_model_vector_size_in_use: value.embedder_model_vector_size_in_use as i64,
        }
    }
}
