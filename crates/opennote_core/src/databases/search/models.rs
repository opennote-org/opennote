use serde::{Deserialize, Serialize};

use crate::models::{block::Block, payload::Payload};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    /// An ordered list of the parent blocks
    /// It starts with the root block and ends with the searched block
    pub path: Vec<Block>,
    /// The payload that has been searched
    pub payload: Payload,
    /// Similarity score
    pub score: f32,
}

impl SearchResult {
    pub fn new(path: Vec<Block>, payload: Payload, score: f32) -> Self {
        Self {
            path,
            payload,
            score,
        }
    }
}
