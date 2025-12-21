use std::sync::Arc;

use anyhow::Result;
use futures::future::join_all;
use log::{error, warn};
use qdrant_client::{
    Qdrant,
    qdrant::{
        Condition, DeletePointsBuilder, Filter, PointStruct, QueryPointsBuilder,
        SearchParamsBuilder, UpsertPointsBuilder,
    },
};
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    configurations::system::{DatabaseConfig, EmbedderConfig},
    embedder::{send_vectorization, send_vectorization_queries},
    identities::storage::UserInformationStorage,
    metadata_storage::MetadataStorage,
    models::{
        document_chunk::{DocumentChunk, DocumentChunkSearchResult},
        document_metadata::DocumentMetadata,
        requests::{SearchDocumentRequest, SearchScope},
    },
};

pub fn preprocess_document(
    title: &str,
    content: &str,
    collection_metadata_id: &str,
    chunk_size: usize,
) -> (DocumentMetadata, Vec<DocumentChunk>, String) {
    // Each chunk will relate to one metadata
    let mut metadata: DocumentMetadata =
        DocumentMetadata::new(title.to_string(), collection_metadata_id.to_string());

    // Each chunk can relate to their metadata with a metadata.id
    let chunks: Vec<DocumentChunk> = DocumentChunk::slice_document_by_period(
        content,
        chunk_size,
        &metadata.metadata_id,
        collection_metadata_id,
    );

    metadata.chunks = chunks.iter().map(|chunk| chunk.id.clone()).collect();
    let metadata_id = metadata.metadata_id.clone();
    (metadata, chunks, metadata_id)
}

// Return a document metadata id on success
pub async fn add_document_chunks_to_database(
    client: &Qdrant,
    metadata_storage: Arc<Mutex<MetadataStorage>>,
    metadata: DocumentMetadata,
    embedder_config: &EmbedderConfig,
    database_config: &DatabaseConfig,
    chunks: Vec<DocumentChunk>,
) -> Result<String> {
    // Vectorize the chunks
    // - Split the chunks into batches
    // - Vectorize batch by batch
    // - Batch is configurable
    let mut metadata_storage = metadata_storage.lock().await;
    let document_metadata_id: String = metadata.metadata_id.clone();

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
        tasks.push(send_vectorization(embedder_config, batch));
    }

    let results: Vec<std::result::Result<Vec<DocumentChunk>, anyhow::Error>> =
        join_all(tasks).await;
    let mut chunks: Vec<DocumentChunk> = Vec::new();
    for result in results {
        let result = result?;
        chunks.extend(result);
    }

    let points: Vec<PointStruct> = chunks
        .into_iter()
        .map(|chunk| PointStruct::from(chunk))
        .collect();

    client
        .upsert_points(UpsertPointsBuilder::new(&database_config.index, points).wait(true))
        .await?;

    metadata_storage.add_document(metadata).await?;

    Ok(document_metadata_id)
}

pub async fn delete_documents_from_database(
    client: &Qdrant,
    metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
    database_config: &DatabaseConfig,
    document_ids: Vec<String>,
) -> Result<()> {
    let mut conditions: Vec<Condition> = Vec::new();
    for id in document_ids.iter() {
        match metadata_storage.remove_document(id).await {
            Some(result) => {
                conditions.push(
                    Condition::matches("document_metadata_id", result.metadata_id)
                );
            }
            None => {
                let message: String =
                    format!("Document {} was not found when trying to delete", id);
                warn!("{}", message);
            }
        };
    }

    match client
        .delete_points(
            DeletePointsBuilder::new(&database_config.index)
                .points(Filter::any(conditions))
                .wait(true),
        )
        .await
    {
        Ok(_) => {}
        Err(error) => error!(
            "Qdrant cannot delete documents {:?} due to {}",
            document_ids, error
        ),
    }

    Ok(())
}

pub async fn intelligent_search_documents(
    client: &Qdrant,
    metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
    user_information_storage: &mut MutexGuard<'_, UserInformationStorage>,
    index: &str,
    embedder_config: &EmbedderConfig,
    query: &SearchDocumentRequest,
) -> Result<Vec<DocumentChunkSearchResult>> {
    // Convert to vec
    let vectors: Vec<Vec<f32>> =
        send_vectorization_queries(embedder_config, &vec![query.query.clone()]).await?;

    let mut conditions: Vec<Condition> = Vec::new();
    let document_metadata_ids: Vec<&String> = match query.scope.search_scope {
        SearchScope::Userspace => {
            let collection_ids: Vec<&String> =
                user_information_storage.get_resource_ids_by_username(&query.scope.id);
            let mut document_metadata_ids: Vec<&String> = Vec::new();

            for id in collection_ids {
                document_metadata_ids.extend(metadata_storage.get_document_ids_by_collection(id));
            }

            document_metadata_ids
        }
        SearchScope::Collection => metadata_storage.get_document_ids_by_collection(&query.scope.id),
        SearchScope::Document => vec![&query.scope.id],
    };

    for id in document_metadata_ids {
        conditions.push(Condition::matches("document_metadata_id", id.to_string()));
    }

    let response = client
        .query(
            QueryPointsBuilder::new(index)
                .using("dense_text_vector")
                .with_payload(true)
                .query(vectors[0].to_owned())
                .limit(query.top_n as u64)
                .filter(Filter::any(conditions))
                .params(SearchParamsBuilder::default().hnsw_ef(128).exact(false)),
        )
        .await?;

    Ok(response
        .result
        .into_iter()
        .map(|item| DocumentChunkSearchResult::from(item))
        .collect())
}

pub async fn search_documents(
    client: &Qdrant,
    metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
    user_information_storage: &mut MutexGuard<'_, UserInformationStorage>,
    index: &str,
    query: &SearchDocumentRequest,
) -> Result<Vec<DocumentChunkSearchResult>> {
    let mut conditions: Vec<Condition> = Vec::new();
    let document_metadata_ids: Vec<&String> = match query.scope.search_scope {
        SearchScope::Userspace => {
            let collection_ids: Vec<&String> =
                user_information_storage.get_resource_ids_by_username(&query.scope.id);
            let mut document_metadata_ids: Vec<&String> = Vec::new();

            for id in collection_ids {
                document_metadata_ids.extend(metadata_storage.get_document_ids_by_collection(id));
            }

            document_metadata_ids
        }
        SearchScope::Collection => metadata_storage.get_document_ids_by_collection(&query.scope.id),
        SearchScope::Document => vec![&query.scope.id],
    };

    for id in document_metadata_ids {
        conditions.push(Condition::matches("document_metadata_id", id.to_string()));
    }

    // conditions.push(Condition::matches_text_any("content", query.query.clone()));

    // let response: ScrollResponse = client
    //     .scroll(
    //         ScrollPointsBuilder::new(index)
    //             .filter(Filter::must(conditions))
    //             .limit(query.top_n as u32)
    //             .build(),
    //     )
    //     .await?;

    let response = client
        .query(
            QueryPointsBuilder::new(index)
                .using("sparse_text_vector")
                .with_payload(true)
                .query(qdrant_client::qdrant::Query::new_nearest(
                    qdrant_client::qdrant::Document {
                        text: query.query.clone(),
                        model: "qdrant/bm25".into(),
                        ..Default::default()
                    },
                ))
                .limit(query.top_n as u64)
                .filter(Filter::any(conditions)),
        )
        .await?;

    Ok(response
        .result
        .into_iter()
        .map(|item| DocumentChunkSearchResult::from(item))
        .collect())
}
