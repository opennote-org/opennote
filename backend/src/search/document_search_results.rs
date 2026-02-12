use std::collections::HashMap;

use local_vector_database::constants::{F_ID, F_METRICS};
use qdrant_client::qdrant::{RetrievedPoint, ScoredPoint};
use serde::{Deserialize, Serialize};

use crate::documents::document_chunk::DocumentChunk;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentChunkSearchResult {
    pub document_title: Option<String>,
    pub collection_title: Option<String>,
    pub document_chunk: DocumentChunk,
    /// Similarity score
    pub score: f32,
}

impl Default for DocumentChunkSearchResult {
    fn default() -> Self {
        Self {
            document_title: None,
            collection_title: None,
            document_chunk: DocumentChunk::default(),
            score: 0.0,
        }
    }
}

impl From<RetrievedPoint> for DocumentChunkSearchResult {
    fn from(value: RetrievedPoint) -> Self {
        Self {
            document_chunk: value.into(),
            ..Default::default()
        }
    }
}

impl From<ScoredPoint> for DocumentChunkSearchResult {
    fn from(value: ScoredPoint) -> Self {
        Self {
            document_chunk: DocumentChunk::from(value.payload),
            score: value.score,
            ..Default::default()
        }
    }
}

impl From<HashMap<String, serde_json::Value>> for DocumentChunkSearchResult {
    fn from(value: HashMap<String, serde_json::Value>) -> Self {
        // the `F_METRICS` is defined at the local vector database
        // however, for compatibility with other potential HashMaps, 
        // we will need to align the fields when we need to incorporate them
        let score = value.get(F_METRICS).unwrap().as_f64().unwrap() as f32;
        
        // remove the fields that are not able to convert to a DocumentChunk
        // the removed ones will be put into the DocumentChunkSearchResult
        let mut value = value;
        value.remove(F_METRICS);
        
        // the `F_ID` is also defined in the local vector database impl 
        // again, need to align the fields when we need to incorporate new HashMaps
        value.remove(F_ID);
        
        Self {
            document_title: None,
            collection_title: None,
            document_chunk: value.into(),
            score,
        }
    }
}
