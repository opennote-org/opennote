//! A list of checkups to run before booting up the program

use anyhow::{Result, anyhow};

use crate::{
    app_state::AppState,
    configurations::system::{Config, EmbedderConfig},
    database::reindex_documents,
    embedder::send_vectorization_queries,
};

pub async fn handshake_embedding_service(config: &EmbedderConfig) -> Result<()> {
    match send_vectorization_queries(
        &config.base_url,
        &config.api_key,
        &config.model,
        &config.encoding_format,
        &vec!["a test string".to_string()],
    )
    .await
    {
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

pub async fn align_embedder_model(config: &Config, app_state: &AppState) -> Result<()> {
    let mut metadata_storage = app_state.metadata_storage.lock().await;

    // This means it is the first time setting up the backend,
    // therefore, we just swap the model name in.
    if metadata_storage.embedder_model_in_use.is_empty()
        && metadata_storage.embedder_model_vector_size_in_use == 0
    {
        metadata_storage.embedder_model_in_use = config.embedder.model.clone();
        metadata_storage.embedder_model_vector_size_in_use = config.embedder.dimensions;
    }

    // This means the embedder model has changed
    if metadata_storage.embedder_model_in_use != config.embedder.model
        || metadata_storage.embedder_model_vector_size_in_use != config.embedder.dimensions
    {
        log::info!("Embedder model has changed. Perform re-indexing. please wait...");
        let client = app_state.database.get_client();
        reindex_documents(client, config).await?;
        log::info!("Re-indexing finished.");
    }

    Ok(())
}
