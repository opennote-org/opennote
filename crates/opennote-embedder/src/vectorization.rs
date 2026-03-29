use anyhow::Result;
use futures::future::join_all;

use opennote_models::{configurations::system::EmbedderConfig, payload::Payload};

use crate::entry::EmbedderEntry;

pub async fn vectorize(
    embedder_entry: &EmbedderEntry,
    embedder_config: &EmbedderConfig,
    chunks: Vec<Payload>,
) -> Result<Vec<Payload>> {
    let mut batches: Vec<Vec<Payload>> = Vec::new();
    let mut batch: Vec<Payload> = Vec::new();
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
        tasks.push(send_vectorization(batch, embedder_entry));
    }

    let results = join_all(tasks).await;
    let mut chunks: Vec<Payload> = Vec::new();
    for result in results {
        let result = result?;
        chunks.extend(result);
    }

    Ok(chunks)
}

pub async fn send_vectorization(
    mut queries: Vec<Payload>,
    embedder_entry: &EmbedderEntry,
) -> Result<Vec<Payload>> {
    let vectors = embedder_entry.embedder.vectorize(&queries).await?;

    for (vector, chunk) in vectors.into_iter().zip(&mut queries) {
        chunk.vector = vector;
    }

    Ok(queries)
}
