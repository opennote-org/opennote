use std::time::Duration;

use anyhow::{anyhow, Result};
use serde_json::{Value, json};

use crate::{configurations::system::EmbedderConfig, models::document_chunk::DocumentChunk};

pub async fn send_vectorization(
    config: &EmbedderConfig,
    mut queries: Vec<DocumentChunk>,
) -> Result<Vec<DocumentChunk>> {
    let vectors: Vec<Vec<f32>> = match send_vectorization_queries(
        config,
        &queries.iter()
            .map(|item| item.content.clone())
            .collect(),
    ).await {
        Ok(result) => result,
        Err(error) => {
            log::error!("Vectorization failed due to {}", error);
            return Err(anyhow!("{}", error));
        }
    };

    for (vector, chunk) in vectors.into_iter().zip(&mut queries) {
        chunk.dense_text_vector = vector;
    }

    Ok(queries)
}

/// TODO:
/// need to create a keep-live mechanism, instead of using a super long timeout
pub async fn send_vectorization_queries(
    config: &EmbedderConfig,
    queries: &Vec<String>,
) -> Result<Vec<Vec<f32>>, anyhow::Error> {
    let client = reqwest::Client::new();
    
    let response = client
        .post(&config.base_url)
        .bearer_auth(&config.api_key)
        .json(&json!(
            {
                "input": queries,
                "model": config.model,
                "encoding_format": config.encoding_format,
                // "dimensions": config.dimensions,
            }
        ))
        .timeout(Duration::from_secs(1000))
        .send().await?;
    
    match response.error_for_status_ref() {
        Ok(_) => {},
        Err(error) => {
            log::error!("Error response body: {}", response.text().await?);
            return Err(anyhow!("Vectorization request has failed. Error: {}", error));
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
