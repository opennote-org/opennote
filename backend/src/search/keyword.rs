use anyhow::Result;
use qdrant_client::{
    Qdrant,
    qdrant::{Condition, Filter, QueryPointsBuilder, ScrollPointsBuilder, ScrollResponse},
};

use crate::{documents::document_chunk::DocumentChunkSearchResult, search::build_search_conditions};

/// This is reserved for sparse vector searches
#[allow(dead_code)]
pub async fn search_documents_with_sparse_vector(
    client: &Qdrant,
    document_metadata_ids: Vec<String>,
    index: &str,
    query: &str,
    top_n: usize,
) -> Result<Vec<DocumentChunkSearchResult>> {
    let conditions: Vec<Condition> = build_search_conditions(document_metadata_ids);

    let response = client
        .query(
            QueryPointsBuilder::new(index)
                .using("sparse_text_vector")
                .with_payload(true)
                .query(qdrant_client::qdrant::Query::new_nearest(
                    qdrant_client::qdrant::Document {
                        text: query.to_string(),
                        model: "qdrant/bm25".into(),
                        ..Default::default()
                    },
                ))
                .limit(top_n as u64)
                .filter(Filter::any(conditions)),
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
    document_metadata_ids: Vec<String>,
    index: &str,
    query: &str,
    top_n: usize,
) -> Result<Vec<DocumentChunkSearchResult>> {
    let conditions: Vec<Condition> = build_search_conditions(document_metadata_ids);

    let response: ScrollResponse = client
        .scroll(
            ScrollPointsBuilder::new(index)
                .filter(
                    Filter {
                        should: conditions,
                        must: vec![Condition::matches_text_any("content", query)],
                        ..Default::default()
                    }
                )
                .limit(top_n as u32)
                .build(),
        )
        .await?;

    Ok(response
        .result
        .into_iter()
        .map(|item| DocumentChunkSearchResult::from(item))
        .collect())
}
