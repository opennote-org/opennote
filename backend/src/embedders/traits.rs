use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::models::payload::Payload;

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

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn vectorize(&self, queries: &Vec<Payload>) -> anyhow::Result<Vec<Vec<f32>>>;
}
