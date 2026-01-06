use std::time::Duration;

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::documents::document_chunk::DocumentChunk;

pub async fn send_vectorization(
    base_url: &str,
    api_key: &str,
    model: &str,
    encoding_format: &str,
    mut queries: Vec<DocumentChunk>,
) -> Result<Vec<DocumentChunk>> {
    let vectors: Vec<Vec<f32>> = match send_vectorization_queries(
        base_url,
        api_key,
        model,
        encoding_format,
        &queries.iter().map(|item| item.content.clone()).collect(),
    )
    .await
    {
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
    base_url: &str,
    api_key: &str,
    model: &str,
    encoding_format: &str,
    queries: &Vec<String>,
) -> Result<Vec<Vec<f32>>, anyhow::Error> {
    let client = reqwest::Client::new();

    let response = client
        .post(base_url)
        .bearer_auth(api_key)
        .json(&json!(
            {
                "input": queries,
                "model": model,
                "encoding_format": encoding_format,
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
            if error_response_body.contains("Please reduce the length of the input.") {
                log::error!(
                    "User had requested a larger chunk than the embedding model can handle. Please set the chunk size smaller"
                );
            } else {
                log::error!("Error response body: {}", error_response_body);
            }

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
