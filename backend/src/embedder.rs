use anyhow::Result;
use futures::future::join_all;

use crate::{
    configurations::system::EmbedderConfig, documents::document_chunk::DocumentChunk,
    embedders::entry::EmbedderEntry,
};

pub async fn vectorize(
    embedder_config: &EmbedderConfig,
    chunks: Vec<DocumentChunk>,
    embedder_entry: &EmbedderEntry,
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
        tasks.push(send_vectorization(batch, embedder_entry));
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
    mut queries: Vec<DocumentChunk>,
    embedder_entry: &EmbedderEntry,
) -> Result<Vec<DocumentChunk>> {
    let vectors = embedder_entry.embedder.vectorize(&queries).await?;

    for (vector, chunk) in vectors.into_iter().zip(&mut queries) {
        chunk.dense_text_vector = vector;
    }

    Ok(queries)
}
