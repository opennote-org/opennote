use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use opennote_models::{
    assets::construct_asset_save_path,
    configurations::{fields::EmbedderConfig, system::SystemConfigurations},
    payload::Payload,
};

use crate::{
    shared::url::{UrlAssetsDownloader, parse_remote_asset_url},
    traits::Embedder,
};

pub struct OpenAICompatible {
    embedder_config: EmbedderConfig,
}

#[derive(Serialize)]
struct EmbeddingsRequest<'a> {
    input: Value,
    model: &'a str,
    encoding_format: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    index: Option<usize>,
    embedding: Vec<f32>,
}

impl OpenAICompatible {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        Ok(Self {
            embedder_config: config.embedder.clone(),
        })
    }
}

#[async_trait]
impl Embedder for OpenAICompatible {
    async fn vectorize(
        &self,
        assets_storage_path: Option<PathBuf>,
        queries: &[Payload],
    ) -> Result<Vec<Vec<f32>>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        let references: Vec<_> = queries.iter().map(Payload::get_asset_reference).collect();
        let has_images = references.iter().any(Option::is_some);

        let input = match has_images {
            true => {
                construct_multimodal_input(assets_storage_path.as_deref(), queries, references)
                    .await?
            }
            false => {
                json!(
                    queries
                        .iter()
                        .map(|query| query.texts.as_str())
                        .collect::<Vec<_>>()
                )
            }
        };

        let client = reqwest::Client::new();

        let response = client
            .post(&self.embedder_config.base_url)
            .bearer_auth(&self.embedder_config.api_key)
            .json(&EmbeddingsRequest {
                input,
                model: &self.embedder_config.model,
                encoding_format: &self.embedder_config.encoding_format,
            })
            .timeout(Duration::from_secs(1000))
            .send()
            .await?;

        match response.error_for_status_ref() {
            Err(error) if has_images => {
                // Validation errors may echo the request, including the encoded images.
                return Err(anyhow!(
                    "Multimodal vectorization failed: {error}. Check that the endpoint and model support image embeddings via input[].content."
                ));
            }
            Err(error) => {
                let body = response.text().await?;
                return Err(anyhow!(
                    "Vectorization request failed: {error}. Message: {body}"
                ));
            }
            Ok(_) => {}
        }

        let EmbeddingsResponse { mut data } = response.json().await?;

        if data.len() != queries.len() {
            return Err(anyhow!(
                "Expected {} remote embeddings, got {}",
                queries.len(),
                data.len()
            ));
        }

        // Keep supporting endpoints that omit all indices and return vectors in input order.
        if data.iter().any(|item| item.index.is_some()) {
            data.sort_unstable_by_key(|item| item.index);

            if data
                .iter()
                .enumerate()
                .any(|(index, item)| item.index != Some(index))
            {
                return Err(anyhow!(
                    "Invalid or missing indices in remote embedding response"
                ));
            }
        }

        Ok(data.into_iter().map(|item| item.embedding).collect())
    }
}

async fn construct_multimodal_input(
    root: Option<&Path>,
    queries: &[Payload],
    references: Vec<Option<String>>,
) -> Result<Value> {
    // Dispatch url downloads first so they run while local assets are read.
    let mut downloader = UrlAssetsDownloader::new();
    let mut url_indices = Vec::new();

    let mut local_references = Vec::new();
    let mut parts = vec![Value::Null; queries.len()];

    for (index, (query, reference)) in queries.iter().zip(references).enumerate() {
        match reference {
            None => parts[index] = json!({"type": "text", "text": query.texts}),
            Some(reference) => match parse_remote_asset_url(&reference) {
                Some(url) => {
                    downloader.start_download_url_asset_task(url).await;
                    url_indices.push(index);
                }
                None => local_references.push((index, reference)),
            },
        }
    }

    if !local_references.is_empty() {
        let root = match root {
            Some(root) => tokio::fs::canonicalize(root).await?,
            None => return Err(anyhow!("No assets storage when embedding local assets")),
        };

        for (index, reference) in local_references {
            let bytes =
                tokio::fs::read(construct_asset_save_path(root.clone(), &reference)).await?;
            parts[index] = encode_image_part(&bytes)?;
        }
    }

    if downloader.has_tasks() {
        let downloaded = downloader.fetch_downloaded_tasks().await?;

        for (index, bytes) in url_indices.into_iter().zip(downloaded) {
            parts[index] = encode_image_part(&bytes)?;
        }
    }

    // OpenRouter-style input: each payload gets its own content object and embedding.
    let inputs = parts
        .into_iter()
        .map(|part| json!({"content": [part]}))
        .collect();

    Ok(Value::Array(inputs))
}

fn encode_image_part(bytes: &[u8]) -> Result<Value> {
    let format = image::guess_format(bytes)?;

    let url = format!(
        "data:{};base64,{}",
        format.to_mime_type(),
        STANDARD.encode(bytes)
    );

    Ok(json!({"type": "image_url", "image_url": {"url": url}}))
}
