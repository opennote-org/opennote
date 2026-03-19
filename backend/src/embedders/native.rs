use crate::configurations::system::EmbedderConfig;
use crate::embedders::native_embedder::embedder::EmbedderTrait;
use crate::embedders::native_embedder::native_embedder::NativeEmbedder;
use crate::{
    configurations::system::Config, documents::document_chunk::DocumentChunk,
    embedders::traits::Embedder,
};
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;

pub struct Native {
    embedder_config: EmbedderConfig,
}

impl Native {
    pub async fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            embedder_config: config.embedder.clone(),
        })
    }
}

#[async_trait]
impl Embedder for Native {
    async fn vectorize(&self, queries: &Vec<DocumentChunk>) -> anyhow::Result<Vec<Vec<f32>>> {
        let native_embedder = NativeEmbedder::new(&self.embedder_config.model)?;

        let inputs: Vec<&str> = queries.iter().map(|item| item.content.as_str()).collect();

        let result = native_embedder.embed(&inputs).await.map_err(|error| {
            log::error!("Vectorization failed due to {}", error);
            anyhow!("{}", error)
        })?;

        Ok(result)
    }
}
