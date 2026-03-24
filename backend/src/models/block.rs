use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::payload::Payload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    /// Parent ID of this block. Root blocks don't have parent ids.
    pub parent_id: Option<String>,
    /// Reserved for soft deletion
    pub is_deleted: bool,
    /// Actual data contained in this block
    pub payloads: Vec<Payload>,
}

impl Block {
    /// Convert from a database model
    pub fn from_model(
        model: crate::entity::blocks::Model,
        payloads: Vec<crate::entity::payloads::Model>,
    ) -> Self {
        Self {
            id: model.id,
            parent_id: model.parent_id,
            is_deleted: model.is_deleted,
            payloads: payloads.into_iter().map(|item| item.into()).collect(),
        }
    }
}
