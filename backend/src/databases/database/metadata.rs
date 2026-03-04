use sqlx::prelude::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct MetadataSettings {
    pub embedder_model_in_use: String,
    pub embedder_model_vector_size_in_use: usize,
}

impl From<crate::database::entity::metadata_settings::Model> for MetadataSettings {
    fn from(value: crate::database::entity::metadata_settings::Model) -> Self {
        Self {
            embedder_model_in_use: value.embedder_model_in_use,
            embedder_model_vector_size_in_use: value.embedder_model_vector_size_in_use as usize,
        }
    }
}

impl From<MetadataSettings> for crate::database::entity::metadata_settings::Model {
    fn from(value: MetadataSettings) -> Self {
        Self {
            id: 1,
            embedder_model_in_use: value.embedder_model_in_use,
            embedder_model_vector_size_in_use: value.embedder_model_vector_size_in_use as i64,
        }
    }
}
