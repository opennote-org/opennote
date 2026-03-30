use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};

use opennote_models::{configurations::system::{SystemConfigurations, EmbedderConfig}, payload::Payload};

use crate::traits::Embedder;

pub struct Remote {
    embedder_config: EmbedderConfig,
}

impl Remote {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        Ok(Self {
            embedder_config: config.embedder.clone(),
        })
    }
}

#[async_trait]
impl Embedder for Remote {
    async fn vectorize(&self, queries: &Vec<Payload>) -> Result<Vec<Vec<f32>>> {
        let client = reqwest::Client::new();

        let response = client
            .post(&self.embedder_config.base_url)
            .bearer_auth(&self.embedder_config.api_key)
            .json(&json!(
                {
                    "input": queries.iter().map(|item| item.texts.clone()).collect::<Vec<String>>(),
                    "model": &self.embedder_config.model,
                    "encoding_format": &self.embedder_config.encoding_format,
                    // "dimensions": config.dimensions,
                }
            ))
            .timeout(Duration::from_secs(1000))
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {}
            Err(error) => {
                let error_response_body: String = response.text().await?;
                return Err(anyhow!(
                    "Vectorization request has failed. Error: {}. Message: {}",
                    error,
                    error_response_body
                ));
            }
        }

        let json_response: Value = response.json::<Value>().await?;

        let vectors: Vec<Vec<f32>> = json_response["data"]
            .as_array()
            .unwrap()
            .into_iter()
            .map(|item| {
                item.as_object().unwrap()["embedding"]
                    .as_array()
                    .unwrap()
                    .into_iter()
                    .map(|item| item.as_f64().unwrap() as f32)
                    .collect()
            })
            .collect();

        Ok(vectors)
    }
}
