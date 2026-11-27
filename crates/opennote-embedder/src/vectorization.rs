use anyhow::Result;
use futures::future::join_all;

use opennote_models::{
    configurations::fields::{EmbedderConfig, assets::AssetsConfigurations},
    payload::Payload,
};

use crate::entry::EmbedderEntry;

pub async fn vectorize(
    embedder_entry: &EmbedderEntry,
    embedder_config: &EmbedderConfig,
    assets_storage_configurations: &AssetsConfigurations,
    payloads: Vec<Payload>,
) -> Result<Vec<Payload>> {
    let mut batches: Vec<Vec<Payload>> = Vec::new();
    let mut batch: Vec<Payload> = Vec::new();
    for payload in payloads {
        if batch.len() == embedder_config.vectorization_batch_size {
            batches.push(batch);
            batch = Vec::new();
        }

        batch.push(payload);
    }

    if !batch.is_empty() {
        batches.push(batch);
    }

    // Record the data entries
    let mut tasks = Vec::new();
    for batch in batches.into_iter() {
        tasks.push(send_vectorization(
            batch,
            assets_storage_configurations.clone(),
            embedder_entry,
        ));
    }

    let results = join_all(tasks).await;
    let mut chunks: Vec<Payload> = Vec::new();
    for result in results {
        let result = result?;
        chunks.extend(result);
    }

    Ok(chunks)
}

/// Swap the vectors into the payloads.
fn swap_vectors_in(vectors: Vec<Vec<f32>>, queries: &mut Vec<Payload>) {
    for (vector, chunk) in vectors.into_iter().zip(queries) {
        chunk.vector = vector;
    }
}

/// It will parallelize the vectorization.
pub async fn send_vectorization(
    mut queries: Vec<Payload>,
    assets_storage_configurations: AssetsConfigurations,
    embedder_entry: &EmbedderEntry,
) -> Result<Vec<Payload>> {
    let vectors = embedder_entry
        .embedder
        .vectorize(
            Some(assets_storage_configurations.assets_storage_path.into()),
            &queries,
        )
        .await?;

    swap_vectors_in(vectors, &mut queries);

    Ok(queries)
}

/// It will vectorize the input as queires.
pub async fn send_query_vectorization(
    mut queries: Vec<Payload>,
    assets_storage_configurations: AssetsConfigurations,
    embedder_entry: &EmbedderEntry,
) -> Result<Vec<Payload>> {
    let vectors = embedder_entry
        .embedder
        .vectorize_query(
            Some(assets_storage_configurations.assets_storage_path.into()),
            &queries,
        )
        .await?;

    swap_vectors_in(vectors, &mut queries);

    Ok(queries)
}
