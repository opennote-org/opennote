use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(from = "String", into = "String")]
pub enum EmbedderProvider {
    /// An embedding model that runs within the app
    Local,
    /// An OpenAI-compatible embeddings endpoint
    OpenAICompatible,
    /// Models from third-party providers
    ThirdParty(String),
}

impl From<String> for EmbedderProvider {
    fn from(provider: String) -> Self {
        match provider.as_str() {
            "local" | "Local" | "native" => Self::Local,
            "openai_compatible" | "OpenAICompatible" | "remote" => Self::OpenAICompatible,
            _ => Self::ThirdParty(provider),
        }
    }
}

impl From<EmbedderProvider> for String {
    fn from(value: EmbedderProvider) -> Self {
        match value {
            EmbedderProvider::Local => "local".into(),
            EmbedderProvider::OpenAICompatible => "openai_compatible".into(),
            EmbedderProvider::ThirdParty(provider) => provider,
        }
    }
}

impl Display for EmbedderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedderProvider::Local => "local",
            EmbedderProvider::OpenAICompatible => "openai_compatible",
            EmbedderProvider::ThirdParty(provider) => provider,
        }
        .fmt(f)
    }
}
