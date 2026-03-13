use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait EmbedderTrait {
    async fn embed(&self, sentences: &[&str]) -> Result<Vec<Vec<f32>>>;
}
