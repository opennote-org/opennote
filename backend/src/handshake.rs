//! Do a service alive check before booting up the entire app

use anyhow::{Result, anyhow};

use crate::{configurations::system::EmbedderConfig, embedder::send_vectorization_queries};

pub async fn handshake_embedding_service(config: &EmbedderConfig) -> Result<()> {
    match send_vectorization_queries(config, &vec!["a test string".to_string()]).await {
        Ok(result) => {
            if let Some(vector) = result.get(0) {
                if !(vector.len() == config.dimensions) {
                    return Err(anyhow!(
                        "Returned vector dimension mismatched the config. Returned vector: {} while being configured to {}",
                        vector.len(),
                        config.dimensions
                    ));
                }
            } else {
                return Err(anyhow!(
                    "Cannot find a vector in the response from the embedder service. Please check whether the embedder service has properly configured / started"
                ));
            }

            Ok(())
        }
        Err(error) => {
            return Err(anyhow!(
                "Cannot handshake with the embedder service: {}",
                error
            ));
        }
    }
}
