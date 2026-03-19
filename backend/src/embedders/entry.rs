use anyhow::Result;
use std::sync::Arc;

use crate::{
    configurations::system::Config,
    embedders::{shared::create_embedder, traits::Embedder},
};

pub struct EmbedderEntry {
    pub embedder: Arc<dyn Embedder>,
}

impl EmbedderEntry {
    pub async fn new(config: &Config) -> Result<Self> {
        let embedder = create_embedder(config).await?;
        Ok(Self { embedder })
    }
}
