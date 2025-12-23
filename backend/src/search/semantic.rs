use anyhow::Result;
use qdrant_client::{
    qdrant::{Condition, Filter, QueryPointsBuilder, ScoredPoint, SearchParamsBuilder}, Qdrant
};

use crate::{
    embedder::send_vectorization_queries, search::build_search_conditions,
};

pub async fn search_documents_semantically(
    client: &Qdrant,
    document_metadata_ids: Vec<String>,
    index: &str,
    query: &str,
    top_n: usize,
    base_url: &str,
    api_key: &str,
    model: &str,
    encoding_format: &str,
) -> Result<Vec<ScoredPoint>> {
    // Convert to vec
    let vectors: Vec<Vec<f32>> = send_vectorization_queries(
        base_url,
        api_key,
        model,
        encoding_format,
        &vec![query.to_string()],
    )
    .await?;
    
    let conditions: Vec<Condition> = build_search_conditions(document_metadata_ids);

    let response = client
        .query(
            QueryPointsBuilder::new(index)
                .using("dense_text_vector")
                .with_payload(true)
                .query(vectors[0].to_owned())
                .limit(top_n as u64)
                .filter(Filter::any(conditions))
                .params(SearchParamsBuilder::default().hnsw_ef(128).exact(false)),
        )
        .await?;

    Ok(response.result)
}
