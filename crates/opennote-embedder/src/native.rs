use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;

use embed::embeddings::embed::{EmbedImage, Embedder as AnythingEmbedder, EmbeddingResult};

use opennote_models::{
    configurations::system::SystemConfigurations, content_type::ContentType, payload::Payload,
};

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
    async fn vectorize(
        &self,
        assets_storage_path: Option<PathBuf>,
        queries: &[Payload],
    ) -> Result<Vec<Vec<f32>>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        let mut multi_modal_inputs = Vec::new();
        let mut multi_modal_inputs_indices = Vec::new();

        let mut inputs = Vec::new();
        let mut inputs_indices = Vec::new();

        for (index, query) in queries.iter().enumerate() {
            match query.content_type {
                ContentType::Image => {
                    if let Some(ref path) = assets_storage_path {
                        if let Some(reference) = query.get_asset_reference() {
                            multi_modal_inputs.push(path.join(reference));
                            multi_modal_inputs_indices.push(index);
                        }
                    }
                }
                _ => {
                    inputs.push(query.texts.as_str());
                    inputs_indices.push(index);
                }
            }
        }

        let results = self.anything_embedder.embed(&inputs, None, None).await?;

        let multi_modal_results = self
            .anything_embedder
            .embed_image_batch(&multi_modal_inputs, None)
            .await?;

        let mut vectors: Vec<Vec<f32>> = vec![Vec::new(); queries.len()];

        for (index, result) in results.into_iter().enumerate() {
            match result {
                EmbeddingResult::DenseVector(vec) => {
                    let original_index = inputs_indices[index];
                    vectors[original_index] = vec;
                }
                EmbeddingResult::MultiVector(_) => {
                    return Err(anyhow!("Multi-vector embeddings are not supported"));
                }
            }
        }

        for (index, result) in multi_modal_results.into_iter().enumerate() {
            match result.embedding {
                EmbeddingResult::DenseVector(vec) => {
                    let original_index = multi_modal_inputs_indices[index];
                    vectors[original_index] = vec;
                }
                EmbeddingResult::MultiVector(_) => {
                    return Err(anyhow!("Multi-vector embeddings are not supported"));
                }
            }
        }

        Ok(vectors)
    }
}
