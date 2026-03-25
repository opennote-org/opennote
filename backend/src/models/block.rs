use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::blocks::{ActiveModel, Model},
    models::payload::Payload,
};

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
    pub fn from_model(model: Model, payloads: Vec<crate::entity::payloads::Model>) -> Self {
        Self {
            id: model.id,
            parent_id: model.parent_id,
            is_deleted: model.is_deleted,
            payloads: payloads.into_iter().map(|item| item.into()).collect(),
        }
    }

    /// Convert multiple database blocks and payloads pairs into blocks
    pub fn from_models(models: Vec<(Model, Vec<crate::entity::payloads::Model>)>) -> Vec<Block> {
        models
            .into_iter()
            .map(|(model, payloads)| Block::from_model(model, payloads))
            .collect()
    }

    /// Convert the block into a database model and a database model of its payloads
    pub fn to_model(self) -> (Model, Vec<crate::entity::payloads::Model>) {
        (
            Model {
                id: self.id,
                parent_id: self.parent_id,
                is_deleted: self.is_deleted,
            },
            self.payloads.into_iter().map(|item| item.into()).collect(),
        )
    }

    /// Convert the multiple blocks into database models
    pub fn to_models(blocks: Vec<Block>) -> Vec<(Model, Vec<crate::entity::payloads::Model>)> {
        blocks.into_iter().map(|item| item.to_model()).collect()
    }

    pub fn to_active_model(self) -> (ActiveModel, Vec<crate::entity::payloads::ActiveModel>) {
        let (block_models, payload_models) = self.to_model();
        (
            ActiveModel {
                id: (),
                parent_id: (),
                is_deleted: (),
            },
            
        )
    }
}
