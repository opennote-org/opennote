use anyhow::{Result, anyhow};
use async_trait::async_trait;
use embed_anything::embeddings::embed::{Embedder as AnythingEmbedder, EmbeddingResult};

pub struct Native {
    embedder_config: EmbedderConfig,
    anything_embedder: AnythingEmbedder,
}

impl Native {
    pub async fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            anything_embedder: AnythingEmbedder::from_pretrained_hf(
                model_id.as_ref(),
                None,
                None,
                None,
            )
            .context("Native embedder initialization failed")?,
            embedder_config: config.embedder.clone(),
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

        let result = self
            .anything_embedder
            .embed(&inputs, None, None)
            .await
            .map_err(|error| {
                log::error!("Vectorization failed due to {}", error);
                anyhow!("{}", error)
            })?;

        let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(results.len());

        for result in results {
            match result {
                EmbeddingResult::DenseVector(vec) => vectors.push(vec),
                EmbeddingResult::MultiVector(_) => {
                    return Err(anyhow!("Multi-vector embeddings are not supported"));
                }
            }
        }

        Ok(result)
    }
}
