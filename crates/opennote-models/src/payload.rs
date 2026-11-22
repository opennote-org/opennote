use chrono::Local;
use pulldown_cmark::{Event, Parser, Tag};
use sea_orm::{ActiveValue::Set, IntoActiveModel};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opennote_entities::payloads::{ActiveModel, Model};

use crate::{assets::is_asset_reference, content_type::ContentType};

/// TODO: do we store dynamic data? like hashmap?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Payload {
    /// A unique identification of its owner block
    #[serde(with = "uuid::serde::compact")]
    pub block_id: Uuid,
    /// A unique identification of this payload
    #[serde(with = "uuid::serde::compact")]
    pub id: Uuid,
    /// When this payload is created
    pub created_at: i64,
    /// Last time this payload is modified
    pub last_modified: i64,
    /// Content type presented in which style. For example, text can be P1 or so.
    pub content_type: ContentType,
    /// Texts stored in payload. When saving jsons, it is recommended to also save a string json for indexing and searching
    pub texts: String,
    /// Bytes stored in payload. Typically, we modalities other than texts, like images and jsons
    pub bytes: Vec<u8>,
    /// Vector representation of the stored texts or bytes
    pub vector: Vec<f32>,
}

impl Payload {
    pub fn new(
        block_id: Uuid,
        content_type: ContentType,
        texts: String,
        bytes: Vec<u8>,
        vector: Vec<f32>,
    ) -> Self {
        let now = Local::now().timestamp();

        Self {
            block_id,
            id: Uuid::new_v4(),
            created_at: now,
            last_modified: now,
            content_type,
            texts,
            bytes,
            vector,
        }
    }

    /// Consume self to create an ActiveModel for updating the database
    pub fn to_active_model(self) -> ActiveModel {
        let model: Model = self.into();
        let mut active_model = model.clone().into_active_model();

        active_model.last_modified = Set(model.last_modified);
        active_model.texts = Set(model.texts);
        active_model.bytes = Set(model.bytes);
        active_model.vector = Set(model.vector);
        active_model.content_type = Set(model.content_type);

        active_model
    }

    /// Get the asset reference from this payload.
    /// Return None when this payload does not store any asset.
    ///
    /// This method would only treat the first image as the asset to return.
    /// But in most cases, it should only store one image per payload.
    pub fn get_asset_reference(&self) -> Option<String> {
        // For now, only image payloads store assets.
        if self.content_type != ContentType::Image {
            return None;
        }

        for event in Parser::new(&self.texts) {
            match event {
                Event::Start(Tag::Image { dest_url, .. }) if is_asset_reference(&dest_url) => {
                    return Some(dest_url.into_string());
                }
                _ => continue,
            }
        }

        None
    }
}

impl From<Model> for Payload {
    fn from(value: Model) -> Self {
        Self {
            block_id: value.block_id,
            id: value.id,
            created_at: value.created_at,
            last_modified: value.last_modified,
            content_type: serde_json::from_str(&value.content_type).unwrap(),
            texts: value.texts,
            bytes: value.bytes,
            vector: serde_json::from_value(value.vector).unwrap(),
        }
    }
}

impl From<Payload> for Model {
    fn from(value: Payload) -> Self {
        Self {
            id: value.id,
            block_id: value.block_id,
            created_at: value.created_at,
            last_modified: value.last_modified,
            texts: value.texts,
            bytes: value.bytes,
            vector: serde_json::to_value(value.vector).unwrap(),
            content_type: serde_json::to_string(&value.content_type).unwrap(),
        }
    }
}

/// Create a payload only for querying, not for storage
pub fn create_query(query: &str) -> Payload {
    Payload {
        block_id: Uuid::max(),
        id: Uuid::new_v4(),
        created_at: 0,
        last_modified: 0,
        content_type: ContentType::Markdown,
        texts: query.to_string(),
        bytes: Vec::new(),
        vector: Vec::new(),
    }
}
