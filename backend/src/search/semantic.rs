use anyhow::Result;
use qdrant_client::{
    Qdrant,
    qdrant::{Condition, Filter, QueryPointsBuilder, ScoredPoint, SearchParamsBuilder},
};

use crate::{
    documents::document_chunk::DocumentChunk,
    embedder::send_vectorization,
    search::build_conditions,
};

pub async fn search_documents_semantically(
    client: &Qdrant,
    document_metadata_ids: Vec<String>,
    index: &str,
    query: &str,
    top_n: usize,
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    encoding_format: &str,
) -> Result<Vec<ScoredPoint>> {
    // Convert to vec
    let chunks: Vec<DocumentChunk> = send_vectorization(
        provider,
        base_url,
        api_key,
        model,
        encoding_format,
        vec![DocumentChunk::new(query.to_owned(), "", "")],
    )
    .await?;

    let conditions: Vec<Condition> = build_conditions(document_metadata_ids);

    let response = client
        .query(
            QueryPointsBuilder::new(index)
                .using("dense_text_vector")
                .with_payload(true)
                .query(chunks[0].dense_text_vector.to_owned())
                .limit(top_n as u64)
                .filter(Filter::any(conditions))
                .params(SearchParamsBuilder::default().hnsw_ef(128).exact(false)),
        )
        .await?;

    Ok(response.result)
}
