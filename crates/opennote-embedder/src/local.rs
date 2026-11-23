use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;

use embed::embeddings::embed::{EmbedImage, Embedder as AnythingEmbedder, EmbeddingResult};

use opennote_models::{
    assets::{construct_asset_save_path, is_asset_reference},
    configurations::system::SystemConfigurations,
    content_type::ContentType,
    payload::Payload,
};

use crate::traits::Embedder;

pub struct Local {
    anything_embedder: AnythingEmbedder,
}

impl Local {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        Ok(Self {
            // TODO: investigate whether this supports baai/bge-m3
            anything_embedder: AnythingEmbedder::from_pretrained_hf(
                &config.embedder.model,
                None,
                None,
                None,
            )
            .context("Local embedder initialization failed")?,
        })
    }

    async fn vectorize_local_text(&self, queries: &[Payload]) -> Result<Vec<Vec<f32>>> {
        let mut inputs = Vec::new();

        for query in queries {
            inputs.push(query.texts.as_str());
        }

        let results = self.anything_embedder.embed(&inputs, None, None).await?;

        let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(queries.len());

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

    async fn vectorize_local_multimodal(
        &self,
        queries: &[Payload],
        assets_storage_path: PathBuf,
    ) -> Result<Vec<Vec<f32>>> {
        let mut multi_modal_inputs = Vec::new();
        let mut multi_modal_inputs_indices = Vec::new();

        let mut inputs = Vec::new();
        let mut inputs_indices = Vec::new();

        for (index, query) in queries.iter().enumerate() {
            match query.content_type {
                ContentType::Image => {
                    if let Some(reference) = query.get_asset_reference() {
                        match is_asset_reference(&reference) {
                            true => multi_modal_inputs.push(construct_asset_save_path(
                                assets_storage_path.clone(),
                                &reference,
                            )),
                            false => multi_modal_inputs.push(reference.into()),
                        }

                        multi_modal_inputs_indices.push(index);
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

#[async_trait]
impl Embedder for Local {
    async fn vectorize(
        &self,
        assets_storage_path: Option<PathBuf>,
        queries: &[Payload],
    ) -> Result<Vec<Vec<f32>>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        match &self.anything_embedder {
            AnythingEmbedder::Text(_embedder) => self.vectorize_local_text(queries).await,
            AnythingEmbedder::MultiModal(_embedder) => match assets_storage_path {
                Some(path) => self.vectorize_local_multimodal(queries, path).await,
                None => {
                    return Err(anyhow!(
                        "Assets storage path is missing when embedding multi-modal asset"
                    ));
                }
            },
            _ => return Err(anyhow!("Can't embed texts with a visual embedder")),
        }
    }
}
