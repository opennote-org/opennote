use anyhow::Result;
use async_trait::async_trait;

use uuid::Uuid;

use crate::database::enums::PayloadQuery;

use opennote_models::payload::Payload;

#[async_trait]
pub trait Payloads {
    async fn create_payloads(&self, payloads: Vec<Payload>) -> Result<()>;

    async fn create_payloads_with_active_models(
        &self,
        active_models: Vec<opennote_entities::payloads::ActiveModel>,
    ) -> Result<()>;

    /// Get payloads with a query filter
    async fn read_payloads(&self, filter: &PayloadQuery) -> Result<Vec<Payload>>;

    /// Update payloads by passing the payloads
    async fn update_payloads(&self, payloads: Vec<Payload>) -> Result<()>;

    /// Update payloads by passing the payload active models
    async fn update_payloads_with_active_models(
        &self,
        active_models: Vec<opennote_entities::payloads::ActiveModel>,
    ) -> Result<()>;

    async fn delete_payloads(&self, filter: &PayloadQuery) -> Result<Vec<Payload>>;

    async fn search(&self, query: &str, payload_ids: &Vec<Uuid>) -> Result<Vec<Payload>>;
}
