use crate::{
    configurations::system::Config,
    embedders::{
        native::Native,
        other::Other,
        remote::Remote,
        traits::{Embedder, EmbedderProvider},
    },
};
use anyhow::Result;
use std::{str::FromStr, sync::Arc};

pub async fn create_embedder(config: &Config) -> Result<Arc<dyn Embedder>> {
    let provider: EmbedderProvider = EmbedderProvider::from_str(&config.embedder.provider)?;

    let embedder: Arc<dyn Embedder> = match provider {
        EmbedderProvider::Native => Arc::new(Native::new(config).await?),
        EmbedderProvider::Remote => Arc::new(Remote::new(config).await?),
        _ => Arc::new(Other::new(config).await?),
    };

    Ok(embedder)
}
