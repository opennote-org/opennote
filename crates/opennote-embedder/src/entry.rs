use anyhow::Result;
use std::sync::Arc;

use opennote_core::configurations::system::Config;

use crate::traits::Embedder;

pub struct EmbedderEntry {
    pub embedder: Arc<dyn Embedder>,
}

impl EmbedderEntry {
    pub async fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            embedder: create_embedder(config).await?,
        })
    }
}
