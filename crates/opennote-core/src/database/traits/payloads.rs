use anyhow::Result;
use async_trait::async_trait;

use opennote_models::payload::Payload;

use crate::database::enums::PayloadQuery;

#[async_trait]
pub trait Payloads {
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
}
