use anyhow::Result;
use std::sync::Arc;

use opennote_models::configurations::system::SystemConfigurations;

use crate::{shared::create_embedder, traits::Embedder};

#[derive(Clone)]
pub struct EmbedderEntry {
    pub embedder: Arc<dyn Embedder>,
}

impl EmbedderEntry {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        Ok(Self {
            embedder: create_embedder(config).await?,
        })
    }
}
