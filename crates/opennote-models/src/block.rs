use sea_orm::ActiveValue::{Set, Unchanged};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opennote_entities::blocks::{ActiveModel, Model};

use crate::payload::Payload;

/// Everything in OpenNote is a block. All data operations MUST be performed on `Block`s for simplicity
/// and efficiency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Block {
    #[serde(with = "uuid::serde::compact")]
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
    pub fn from_model(model: Model, payloads: Vec<opennote_entities::payloads::Model>) -> Self {
        Self {
            id: model.id,
            parent_id: model.parent_id,
            is_deleted: model.is_deleted,
            payloads: payloads.into_iter().map(|item| item.into()).collect(),
        }
    }

    /// Convert multiple database blocks and payloads pairs into blocks
    pub fn from_models(
        models: Vec<(Model, Vec<opennote_entities::payloads::Model>)>,
    ) -> Vec<Block> {
        models
            .into_iter()
            .map(|(model, payloads)| Block::from_model(model, payloads))
            .collect()
    }

    /// Convert the block into a database model and a database model of its payloads
    pub fn to_model(self) -> (Model, Vec<opennote_entities::payloads::Model>) {
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
    pub fn to_models(blocks: Vec<Block>) -> Vec<(Model, Vec<opennote_entities::payloads::Model>)> {
        blocks.into_iter().map(|item| item.to_model()).collect()
    }

    /// Consume self to create an ActiveModel for updating the database
    pub fn to_active_model(self) -> (ActiveModel, Vec<opennote_entities::payloads::ActiveModel>) {
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
    ) -> Vec<(ActiveModel, Vec<opennote_entities::payloads::ActiveModel>)> {
        blocks
            .into_iter()
            .map(|item| item.to_active_model())
            .collect()
    }
}
