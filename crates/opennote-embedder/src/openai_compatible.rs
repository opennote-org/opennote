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

use crate::traits::Embedder;

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
            true => match assets_storage_path {
                None => return Err(anyhow!("No assets storage when embedding assets")),
                Some(root) => construct_multimodal_input(&root, queries, references).await?,
            },
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
    root: &Path,
    queries: &[Payload],
    references: Vec<Option<String>>,
) -> Result<Value> {
    let root = tokio::fs::canonicalize(root).await?;

    let mut inputs = Vec::with_capacity(queries.len());

    // OpenRouter-style input: each payload gets its own content object and embedding.
    for (query, reference) in queries.iter().zip(references) {
        let part = match reference {
            Some(reference) => {
                let path = construct_asset_save_path(root.clone(), &reference);

                let bytes = tokio::fs::read(path).await?;
                let format = image::guess_format(&bytes)?;

                let url = format!(
                    "data:{};base64,{}",
                    format.to_mime_type(),
                    STANDARD.encode(bytes)
                );

                json!({"type": "image_url", "image_url": {"url": url}})
            }
            None => json!({"type": "text", "text": query.texts}),
        };
        inputs.push(json!({"content": [part]}));
    }

    Ok(Value::Array(inputs))
}
