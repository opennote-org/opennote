use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;

use embed::embeddings::embed::{Embedder as AnythingEmbedder, EmbeddingResult};

use opennote_models::{configurations::system::SystemConfigurations, payload::Payload};

use crate::traits::Embedder;

pub struct Native {
    anything_embedder: AnythingEmbedder,
}

impl Native {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        Ok(Self {
            // TODO: investigate whether this supports baai/bge-m3
            anything_embedder: AnythingEmbedder::from_pretrained_hf(
                &config.embedder.model,
                None,
                None,
                None,
            )
            .context("Native embedder initialization failed")?,
        })
    }
}

#[async_trait]
impl Embedder for Native {
    async fn vectorize(&self, queries: &Vec<Payload>) -> Result<Vec<Vec<f32>>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        let inputs: Vec<&str> = queries.iter().map(|item| item.texts.as_str()).collect();

        let results = self
            .anything_embedder
            .embed(&inputs, None, None)
            .await
            .map_err(|error| anyhow!("{}", error))?;

        let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(results.len());

        for result in results {
            match result {
                EmbeddingResult::DenseVector(vec) => vectors.push(vec),
                EmbeddingResult::MultiVector(_) => {
                    return Err(anyhow!("Multi-vector embeddings are not supported"));
                }
            }
        }

        Ok(vectors)
    }
}
