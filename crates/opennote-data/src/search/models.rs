use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSearchResult {
    pub block_id: Uuid,
    pub payload_id: Uuid,
    /// Similarity score
    pub score: f32,
}
