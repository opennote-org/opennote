use std::collections::HashMap;

use qdrant_client::qdrant::{Condition, RetrievedPoint, ScoredPoint};
use serde::{Deserialize, Serialize};

use crate::documents::{
    collection_metadata::CollectionMetadata, document_chunk::DocumentChunkSearchResult,
    document_metadata::DocumentMetadata,
};

pub mod keyword;
pub mod semantic;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchScopeIndicator {
    pub search_scope: SearchScope,
    pub id: String,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    Document,
    Collection,
    Userspace,
}

pub fn build_conditions(document_metadata_ids: Vec<String>) -> Vec<Condition> {
    document_metadata_ids
        .into_iter()
        .map(|id| Condition::matches("document_metadata_id", id.to_string()))
        .collect()
}

pub fn build_search_results(
    scored_points: Option<Vec<ScoredPoint>>,
    retrieved_points: Option<Vec<RetrievedPoint>>,
    collection_metadatas_from_storage: &HashMap<String, CollectionMetadata>,
    document_metadatas_from_storage: &HashMap<String, DocumentMetadata>,
) -> Vec<DocumentChunkSearchResult> {
    let mut results = Vec::new();

    if let Some(points) = scored_points {
        for point in points {
            let mut result: DocumentChunkSearchResult = DocumentChunkSearchResult::from(point);

            if let Some(document_metadata) =
                document_metadatas_from_storage.get(&result.document_chunk.document_metadata_id)
            {
                result.document_title = Some(document_metadata.title.clone());
            }

            if let Some(collection_metadata) =
                collection_metadatas_from_storage.get(&result.document_chunk.collection_metadata_id)
            {
                result.collection_title = Some(collection_metadata.title.clone());
            }

            results.push(result);
        }
    }

    if let Some(points) = retrieved_points {
        for point in points {
            let mut result: DocumentChunkSearchResult = DocumentChunkSearchResult::from(point);

            if let Some(document_metadata) =
                document_metadatas_from_storage.get(&result.document_chunk.document_metadata_id)
            {
                result.document_title = Some(document_metadata.title.clone());
            }

            if let Some(collection_metadata) =
                collection_metadatas_from_storage.get(&result.document_chunk.collection_metadata_id)
            {
                result.collection_title = Some(collection_metadata.title.clone());
            }

            results.push(result);
        }
    }

    results
}
