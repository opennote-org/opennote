use crate::embedder::EmbedderTrait;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use embed_anything::embeddings::embed::{Embedder as AnythingEmbedder, EmbeddingResult};

pub struct LocalEmbedder {
    anything_embedder: AnythingEmbedder,
}

impl LocalEmbedder {
    pub fn new(model_id: impl AsRef<str>) -> Result<Self> {
        let anything_embedder =
            AnythingEmbedder::from_pretrained_hf(model_id.as_ref(), None, None, None)
                .context("Local embedder initialization failed")?;

        Ok(Self { anything_embedder })
    }

    async fn embed_inner(&self, sentences: &[&str]) -> Result<Vec<Vec<f32>>> {
        if sentences.is_empty() {
            return Ok(vec![]);
        }

        let results = self.anything_embedder.embed(sentences, None, None).await?;

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

#[async_trait]
impl EmbedderTrait for LocalEmbedder {
    async fn embed(&self, sentences: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.embed_inner(sentences).await
    }
}
