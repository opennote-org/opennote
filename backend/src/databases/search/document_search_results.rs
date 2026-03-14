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
