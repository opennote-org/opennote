use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;

use embed::embeddings::embed::{EmbedImage, Embedder as AnythingEmbedder, EmbeddingResult};

use crate::{
    shared::url::{UrlAssetsDownloader, parse_remote_asset_url},
    traits::Embedder,
};
use opennote_models::{
    assets::construct_asset_save_path, configurations::system::SystemConfigurations,
    content_type::ContentType, payload::Payload,
};

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

    /// Since we currently only have embeddinggemma for multimodal embedding,
    /// this function now only works for that model specifically.
    ///
    /// For future models, we need to adapt them differently.
    async fn vectorize_local_multimodal(
        &self,
        queries: &[Payload],
        assets_storage_path: PathBuf,
    ) -> Result<Vec<Vec<f32>>> {
        // Local multi modal assets.
        //
        // The indices are used to maintain the position of each input.
        let mut multi_modal_inputs = Vec::new();
        let mut multi_modal_inputs_indices = Vec::new();

        // Url assets that need to be downloaded first
        let mut multi_modal_bytes_inputs = UrlAssetsDownloader::new();
        let mut multi_modal_bytes_inputs_indices = Vec::new();

        // Texts
        let mut inputs = Vec::new();
        let mut inputs_indices = Vec::new();

        for (index, query) in queries.iter().enumerate() {
            match query.content_type {
                ContentType::Image => {
                    if let Some(reference) = query.get_asset_reference() {
                        match parse_remote_asset_url(&reference) {
                            Some(url) => {
                                multi_modal_bytes_inputs
                                    .start_download_url_asset_task(url)
                                    .await;
                                multi_modal_bytes_inputs_indices.push(index);
                                continue;
                            }
                            None => {}
                        }

                        multi_modal_inputs.push(construct_asset_save_path(
                            assets_storage_path.clone(),
                            &reference,
                        ));

                        multi_modal_inputs_indices.push(index);
                    }
                }
                _ => {
                    inputs.push(format!("title: none | text: {}", query.texts));
                    inputs_indices.push(index);
                }
            }
        }

        let multi_modal_bytes_results = match multi_modal_bytes_inputs.has_tasks() {
            false => Vec::new(),
            true => {
                self.anything_embedder
                    .embed_image_bytes(
                        &multi_modal_bytes_inputs.fetch_downloaded_tasks().await?,
                        None,
                    )
                    .await?
            }
        };

        let inputs: Vec<&str> = inputs.iter().map(|item| item.as_str()).collect();
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

        for (index, result) in multi_modal_bytes_results.into_iter().enumerate() {
            match result.embedding {
                EmbeddingResult::DenseVector(vec) => {
                    let original_index = multi_modal_bytes_inputs_indices[index];
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

    async fn vectorize_query(
        &self,
        _assets_storage_path: Option<PathBuf>,
        queries: &[Payload],
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut queries = queries.to_owned();

        for query in queries.iter_mut() {
            query.texts = format!("{}{}", "task: search result | query: ", query.texts)
        }

        self.vectorize_local_text(&queries).await
    }
}
