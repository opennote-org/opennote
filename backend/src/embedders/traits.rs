use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::documents::document_chunk::DocumentChunk;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum EmbedderProvider {
    Native,
    Remote,
    Other(String),
}

impl From<String> for EmbedderProvider {
    fn from(provider: String) -> Self {
        match provider.as_str() {
            "native" => Self::Native,
            "remote" => Self::Remote,
            _ => Self::Other(provider),
        }
    }
}

impl From<EmbedderProvider> for String {
    fn from(value: EmbedderProvider) -> Self {
        match value {
            EmbedderProvider::Native => "native".into(),
            EmbedderProvider::Remote => "remote".into(),
            EmbedderProvider::Other(provider) => provider,
        }
    }
}

impl fmt::Display for EmbedderProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbedderProvider::Native => "native",
            EmbedderProvider::Remote => "remote",
            EmbedderProvider::Other(provider) => provider,
        }
        .fmt(f)
    }
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn vectorize(&self, queries: &Vec<DocumentChunk>) -> anyhow::Result<Vec<Vec<f32>>>;
}
