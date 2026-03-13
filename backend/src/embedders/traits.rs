use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

use crate::documents::document_chunk::DocumentChunk;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedderProvider {
    Native,
    Remote,
}

impl Display for EmbedderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => f.write_str("native"),
            Self::Remote => f.write_str("remote"),
        }
    }
}

impl FromStr for EmbedderProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "native" => Ok(EmbedderProvider::Native),
            "remote" => Ok(EmbedderProvider::Remote),
            _ => Err(anyhow!("Unknown embedder provider: {}", s)),
        }
    }
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn vectorize(&self, queries: &Vec<DocumentChunk>) -> anyhow::Result<Vec<Vec<f32>>>;
}
