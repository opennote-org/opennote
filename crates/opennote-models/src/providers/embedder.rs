use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum EmbedderProvider {
    /// The embedder model embedded by this app
    Native,
    /// Self-hosted remote embedder model
    Remote,
    /// Models from third-party providers
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

impl Display for EmbedderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedderProvider::Native => "native",
            EmbedderProvider::Remote => "remote",
            EmbedderProvider::Other(provider) => provider,
        }
        .fmt(f)
    }
}
