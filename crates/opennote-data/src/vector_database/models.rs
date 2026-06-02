use serde::{Deserialize, Serialize};

use opennote_models::payload::Payload;

/// A payload to be stored in database for search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct IndexPayload {
    pub payload_id: String,
    pub block_id: String,
    pub vector: Vec<f32>,
}

impl From<Payload> for IndexPayload {
    fn from(value: Payload) -> Self {
        Self {
            payload_id: value.id.to_string(),
            block_id: value.block_id.to_string(),
            vector: value.vector,
        }
    }
}
