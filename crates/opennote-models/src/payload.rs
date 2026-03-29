use sea_orm::{ActiveValue::Set, IntoActiveModel};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opennote_entities::payloads::{ActiveModel, Model};

use crate::content_type::ContentType;

/// Next: do we store dynamic data? like hashmap?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Payload {
    /// A unique identification of its owner block
    pub block_id: Uuid,
    /// A unique identification of this payload
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
    /// Texts stored in payload. When saving jsons, it is recommended to also save a string json for indexing for searching
    pub texts: String,
    /// Bytes stored in payload. Typically, we modalities other than texts, like images and jsons
    pub bytes: Vec<u8>,
    /// Vector representation of the stored texts or bytes
    pub vector: Vec<f32>,
}

impl Payload {
    /// Consume self to create an ActiveModel for updating the database
    pub fn to_active_model(self) -> ActiveModel {
        let model: Model = self.into();
        let mut active_model = model.clone().into_active_model();

        active_model.order_row = Set(model.order_row);
        active_model.order_column = Set(model.order_column);
        active_model.last_modified = Set(model.last_modified);
        active_model.texts = Set(model.texts);
        active_model.bytes = Set(model.bytes);
        active_model.vector = Set(model.vector);
        active_model.content_type = Set(model.content_type);

        active_model
    }
}

impl From<Model> for Payload {
    fn from(value: Model) -> Self {
        Self {
            block_id: value.block_id,
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

impl Into<Model> for Payload {
    fn into(self) -> Model {
        Model {
            id: self.id,
            block_id: self.block_id,
            order_row: self.order_row,
            order_column: self.order_column,
            created_at: self.created_at,
            last_modified: self.last_modified,
            texts: self.texts,
            bytes: self.bytes,
            vector: serde_json::to_value(self.vector).unwrap(),
            content_type: serde_json::to_string(&self.content_type).unwrap(),
        }
    }
}

/// Create a payload only for querying, not for storage
pub fn create_query(query: &str) -> Payload {
    Payload {
        block_id: Uuid::max(),
        id: Uuid::new_v4(),
        order_row: 0,
        order_column: 0,
        created_at: 0,
        last_modified: 0,
        content_type: ContentType::Body,
        texts: query.to_string(),
        bytes: Vec::new(),
        vector: Vec::new(),
    }
}
