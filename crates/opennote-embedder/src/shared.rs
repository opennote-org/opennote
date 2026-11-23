use std::sync::Arc;

use anyhow::Result;

use opennote_models::{
    configurations::system::SystemConfigurations, providers::embedder::EmbedderProvider,
};

use crate::{
    local::Local, openai_compatible::OpenAICompatible, third_party::ThirdParty, traits::Embedder,
};

pub async fn create_embedder(config: &SystemConfigurations) -> Result<Arc<dyn Embedder>> {
    let embedder: Arc<dyn Embedder> = match &config.embedder.provider {
        EmbedderProvider::Local => Arc::new(Local::new(config).await?),
        EmbedderProvider::OpenAICompatible => Arc::new(OpenAICompatible::new(config).await?),
        EmbedderProvider::ThirdParty(_) => Arc::new(ThirdParty::new(config).await?),
    };

    Ok(embedder)
}
