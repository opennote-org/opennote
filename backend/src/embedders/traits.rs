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
    fn from(s: String) -> Self {
        match s.as_str() {
            "native" => Self::Native,
            "remote" => Self::Remote,
            _ => Self::Other(s),
        }
    }
}

impl From<EmbedderProvider> for String {
    fn from(value: EmbedderProvider) -> Self {
        match value {
            EmbedderProvider::Native => "native".into(),
            EmbedderProvider::Remote => "remote".into(),
            EmbedderProvider::Other(s) => s,
        }
    }
}

impl fmt::Display for EmbedderProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbedderProvider::Native => "native",
            EmbedderProvider::Remote => "remote",
            EmbedderProvider::Other(s) => s,
        }
        .fmt(f)
    }
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn vectorize(&self, queries: &Vec<DocumentChunk>) -> anyhow::Result<Vec<Vec<f32>>>;
}
