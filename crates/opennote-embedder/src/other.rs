use async_trait::async_trait;
use anyhow::Result;

use opennote_core::configurations::system::Config;

use crate::traits::Embedder;

pub struct Other {
    embedder_config: EmbedderConfig,
}

impl Other {
    pub async fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            embedder_config: config.embedder.clone(),
        })
    }
}

#[async_trait]
impl Embedder for Other {
    async fn vectorize(&self, queries: &Vec<Payload>) -> anyhow::Result<Vec<Vec<f32>>> {
        let client: catsu::Client = catsu::Client::new()?;

        let response: catsu::EmbedResponse = client
            .embed_with_api_key(
                &self.embedder_config.model,
                queries.iter().map(|item| item.texts.clone()).collect(),
                None,
                None,
                Some(&self.embedder_config.provider.to_string().as_str()),
                Some(self.embedder_config.api_key.to_owned()),
            )
            .await?;

        Ok(response.embeddings)
    }
}
