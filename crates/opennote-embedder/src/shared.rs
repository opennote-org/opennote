use std::sync::Arc;

use anyhow::Result;

use opennote_models::{configurations::system::SystemConfigurations, providers::embedder::EmbedderProvider};

use crate::{native::Native, other::Other, remote::Remote, traits::Embedder};

pub async fn create_embedder(config: &SystemConfigurations) -> Result<Arc<dyn Embedder>> {
    let embedder: Arc<dyn Embedder> = match &config.embedder.provider {
        EmbedderProvider::Native => Arc::new(Native::new(config).await?),
        EmbedderProvider::Remote => Arc::new(Remote::new(config).await?),
        _ => Arc::new(Other::new(config).await?),
    };

    Ok(embedder)
}
