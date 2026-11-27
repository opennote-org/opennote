use std::path::PathBuf;

use async_trait::async_trait;

use opennote_models::payload::Payload;

#[async_trait]
pub trait Embedder: Send + Sync {
    /// In assets storage path, it should include where an asset can be loaded from.
    /// The function will load assets from the specified path along with the references that each payload stored.
    async fn vectorize(
        &self,
        assets_storage_path: Option<PathBuf>,
        queries: &[Payload],
    ) -> anyhow::Result<Vec<Vec<f32>>>;

    /// This needs to be implemented when the model requires a specific format for queries.
    /// For example, gemma requires a template for queries for better multimodal searches.
    async fn vectorize_query(
        &self,
        assets_storage_path: Option<PathBuf>,
        queries: &[Payload],
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        self.vectorize(assets_storage_path, queries).await
    }
}
