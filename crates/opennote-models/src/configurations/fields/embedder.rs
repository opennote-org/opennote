use serde::{Deserialize, Serialize};

use crate::{providers::embedder::EmbedderProvider, traits::CompareNecessaryChanges};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct EmbedderConfig {
    /// Provider of the embedding model
    pub provider: EmbedderProvider,

    /// base url of your local embedder service.
    pub base_url: String,

    /// Model name of the embedding model
    pub model: String,

    /// Larger number will make the vectorization faster,
    /// but try reducing the number to prevent overflowing the API
    pub vectorization_batch_size: usize,

    /// Dimension of the embedding model
    pub dimensions: usize,

    /// Usually this is a float
    pub encoding_format: String,

    /// API key of the model
    pub api_key: String,
}

impl CompareNecessaryChanges<EmbedderConfig> for EmbedderConfig {
    /// Compare the two configs for whether the key fields have changed
    fn compare_necessary_changes(&self, another: &EmbedderConfig) -> bool {
        if another.model != self.model
            || another.dimensions != self.dimensions
            || another.encoding_format != self.encoding_format
        {
            return true;
        }

        false
    }
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            provider: EmbedderProvider::Native,
            base_url: "".to_string(),
            model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            vectorization_batch_size: 100, // How many vectorization tasks at a time
            dimensions: 384, // sentence-transformers/all-MiniLM-L6-v2 is a 1024 dimensional model
            encoding_format: "float".to_string(),
            api_key: "".to_string(),
        }
    }
}
