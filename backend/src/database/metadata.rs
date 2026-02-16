use sqlx::prelude::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct MetadataSettings {
    pub embedder_model_in_use: String,
    pub embedder_model_vector_size_in_use: usize,
}
