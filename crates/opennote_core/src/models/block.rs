use sea_orm::ActiveValue::{Set, Unchanged};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::blocks::{ActiveModel, Model},
    models::payload::Payload,
};

/// Everything in OpenNote is a block. All data operations MUST be performed on `Block`s for simplicity
/// and efficiency.
///
/// TODO: Also make configurations as Block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Block {
    pub id: Uuid,
    /// Parent ID of this block. Root blocks don't have parent ids.
    pub parent_id: Option<Uuid>,
    /// Reserved for soft deletion
    pub is_deleted: bool,
    /// Actual data contained in this block
    pub payloads: Vec<Payload>,
}

impl Block {
    /// Convert from a database model
    pub fn from_model(model: Model, payloads: Vec<crate::entities::payloads::Model>) -> Self {
        Self {
            id: model.id,
            parent_id: model.parent_id,
            is_deleted: model.is_deleted,
            payloads: payloads.into_iter().map(|item| item.into()).collect(),
        }
    }

    /// Convert multiple database blocks and payloads pairs into blocks
    pub fn from_models(models: Vec<(Model, Vec<crate::entities::payloads::Model>)>) -> Vec<Block> {
        models
            .into_iter()
            .map(|(model, payloads)| Block::from_model(model, payloads))
            .collect()
    }

    /// Convert the block into a database model and a database model of its payloads
    pub fn to_model(self) -> (Model, Vec<crate::entities::payloads::Model>) {
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
    pub fn to_models(blocks: Vec<Block>) -> Vec<(Model, Vec<crate::entities::payloads::Model>)> {
        blocks.into_iter().map(|item| item.to_model()).collect()
    }

    /// Consume self to create an ActiveModel for updating the database
    pub fn to_active_model(self) -> (ActiveModel, Vec<crate::entities::payloads::ActiveModel>) {
        (
            ActiveModel {
                id: Unchanged(self.id),
                parent_id: Set(self.parent_id),
                is_deleted: Set(self.is_deleted),
            },
            self.payloads
                .into_iter()
                .map(|item| item.to_active_model())
                .collect(),
        )
    }

    pub fn to_active_models(
        blocks: Vec<Block>,
    ) -> Vec<(ActiveModel, Vec<crate::entities::payloads::ActiveModel>)> {
        blocks
            .into_iter()
            .map(|item| item.to_active_model())
            .collect()
    }
}
