use async_trait::async_trait;

use opennote_models::payload::Payload;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn vectorize(&self, queries: &[Payload]) -> anyhow::Result<Vec<Vec<f32>>>;
}
