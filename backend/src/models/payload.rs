use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::content_type::ContentType;

/// Next: do we store dynamic data? like hashmap?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub id: Uuid,
    /// The position (row) of the payload on a block.
    pub order_row: i64,
    /// The position (column) of the payload on a block.
    pub order_column: i64,
    /// When this payload is created
    pub created_at: i64,
    /// Last time this payload is modified
    pub last_modified: i64,
    /// Content type presented in which style. For example, text can be P1 or so.
    pub content_type: ContentType,
    /// Texts stored in payload
    pub texts: String,
    /// Bytes stored in payload. Typically, we modalities other than texts, like images
    pub bytes: Vec<u8>,
    /// Vector representation of the stored texts or bytes
    pub vector: Vec<f32>,
}

impl From<crate::entity::payloads::Model> for Payload {
    fn from(value: crate::entity::payloads::Model) -> Self {
        Self {
            id: value.id,
            order_row: value.order_row,
            order_column: value.order_column,
            created_at: value.created_at,
            last_modified: value.last_modified,
            content_type: serde_json::from_str(&value.content_type).unwrap(),
            texts: value.texts,
            bytes: value.bytes,
            vector: serde_json::from_value(value.vector).unwrap(),
        }
    }
}
