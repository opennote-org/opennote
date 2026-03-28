use anyhow::Result;
use async_trait::async_trait;

use crate::{databases::database::enums::PayloadQuery, models::payload::Payload};

#[async_trait]
pub trait Payloads {
    /// Get payloads with a query filter
    async fn read_payloads(&self, filter: &PayloadQuery) -> Result<Vec<Payload>>;

    /// Update payloads by passing the payloads
    async fn update_payloads(&self, payloads: Vec<Payload>) -> Result<()>;

    /// Update payloads by passing the payload active models
    async fn update_payloads_with_active_models(
        &self,
        active_models: Vec<crate::entities::payloads::ActiveModel>,
    ) -> Result<()>;

    async fn delete_payloads(&self, filter: &PayloadQuery) -> Result<Vec<Payload>>;
}
