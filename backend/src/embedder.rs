use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::future::join_all;
use serde_json::{Value, json};

use crate::{configurations::system::EmbedderConfig, documents::document_chunk::DocumentChunk};

pub async fn vectorize(
    embedder_config: &EmbedderConfig,
    chunks: Vec<DocumentChunk>,
) -> Result<Vec<DocumentChunk>> {
    let mut batches: Vec<Vec<DocumentChunk>> = Vec::new();
    let mut batch: Vec<DocumentChunk> = Vec::new();
    for chunk in chunks {
        if batch.len() == embedder_config.vectorization_batch_size {
            batches.push(batch);
            batch = Vec::new();
        }

        batch.push(chunk);
    }

    if !batch.is_empty() {
        batches.push(batch);
    }

    // Record the data entries
    let mut tasks = Vec::new();
    for batch in batches.into_iter() {
        tasks.push(send_vectorization(
            &embedder_config.provider,
            &embedder_config.base_url,
            &embedder_config.api_key,
            &embedder_config.model,
            &embedder_config.encoding_format,
            batch,
        ));
    }

    let results: Vec<std::result::Result<Vec<DocumentChunk>, anyhow::Error>> =
        join_all(tasks).await;
    let mut chunks: Vec<DocumentChunk> = Vec::new();
    for result in results {
        let result = result?;
        chunks.extend(result);
    }

    Ok(chunks)
}

pub async fn send_vectorization(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    encoding_format: &str,
    mut queries: Vec<DocumentChunk>,
) -> Result<Vec<DocumentChunk>> {
    let vectors: Vec<Vec<f32>> = if !provider.is_empty() {
        match send_vectorization_queries_to_multiple_providers(
            api_key, model, provider, None, &queries,
        )
        .await
        {
            Ok(results) => results,
            Err(error) => {
                log::error!("Vectorization failed due to {}", error);
                return Err(anyhow!("{}", error));
            }
        }
    } else {
        match send_vectorization_queries(base_url, api_key, model, encoding_format, &queries).await
        {
            Ok(result) => result,
            Err(error) => {
                log::error!("Vectorization failed due to {}", error);
                return Err(anyhow!("{}", error));
            }
        }
    };

    for (vector, chunk) in vectors.into_iter().zip(&mut queries) {
        chunk.dense_text_vector = vector;
    }

    Ok(queries)
}

pub async fn send_vectorization_queries_to_multiple_providers(
    api_key: &str,
    model: &str,
    provider: &str,
    dimensions: Option<usize>,
    queries: &Vec<DocumentChunk>,
) -> Result<Vec<Vec<f32>>, anyhow::Error> {
    let client: catsu::Client = catsu::Client::new()?;

    let response: catsu::EmbedResponse = client
        .embed_with_api_key(
            model,
            queries.iter().map(|item| item.content.clone()).collect(),
            None,
            dimensions.map(|num| num as u32),
            Some(provider),
            Some(api_key.to_owned()),
        )
        .await?;

    Ok(response.embeddings)
}

/// TODO:
/// need to create a keep-live mechanism, instead of using a super long timeout
pub async fn send_vectorization_queries(
    base_url: &str,
    api_key: &str,
    model: &str,
    encoding_format: &str,
    queries: &Vec<DocumentChunk>,
) -> Result<Vec<Vec<f32>>, anyhow::Error> {
    let client = reqwest::Client::new();

    let response = client
        .post(base_url)
        .bearer_auth(api_key)
        .json(&json!(
            {
                "input": queries.iter().map(|item| item.content.clone()).collect::<Vec<String>>(),
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
