//! A list of checkups to run before booting up the program

use anyhow::{Result, anyhow};

use crate::{
    app_state::AppState,
    configurations::system::{Config, EmbedderConfig},
    documents::document_chunk::DocumentChunk,
    embedder::send_vectorization,
    embedders::entry::EmbedderEntry,
};

pub async fn handshake_embedding_service(
    config: &EmbedderConfig,
    embedder_entry: &EmbedderEntry,
) -> Result<()> {
    match send_vectorization(vec![DocumentChunk::default()], embedder_entry).await {
        Ok(result) => {
            if let Some(vector) = result.get(0) {
                if !(vector.dense_text_vector.len() == config.dimensions) {
                    return Err(anyhow!(
                        "Returned vector dimension mismatched the config. Returned vector: {} while being configured to {}",
                        vector.dense_text_vector.len(),
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
    let mut metadata_settings = app_state
        .databases_layer_entry
        .database
        .get_metadata_settings()
        .await?;

    // This means the embedder model has changed
    if !metadata_settings.embedder_model_in_use.is_empty()
        && (metadata_settings.embedder_model_in_use != config.embedder.model
            || metadata_settings.embedder_model_vector_size_in_use != config.embedder.dimensions)
    {
        log::info!("Embedder model has changed. Perform re-indexing. please wait...");
        app_state
            .databases_layer_entry
            .vector_database
            .reindex_documents(
                config,
                &app_state.databases_layer_entry.database,
                &app_state.embedder_entry,
            )
            .await?;
        log::info!("Re-indexing finished.");
    }

    metadata_settings.embedder_model_in_use = config.embedder.model.clone();
    metadata_settings.embedder_model_vector_size_in_use = config.embedder.dimensions;
    app_state
        .databases_layer_entry
        .database
        .update_metadata_settings(metadata_settings)
        .await?;

    Ok(())
}

pub async fn align_vector_database(config: &Config, app_state: &AppState) -> Result<()> {
    let mut metadata_settings = app_state
        .databases_layer_entry
        .database
        .get_metadata_settings()
        .await?;

    // This means the embedder model has changed
    if !metadata_settings.vector_database_in_use.is_empty()
        && (metadata_settings.vector_database_in_use != config.vector_database.provider.to_string())
    {
        log::info!("Vector database has changed. Perform re-indexing. please wait...");
        app_state
            .databases_layer_entry
            .recover(&config.vector_database.index, config.embedder.dimensions)
            .await?;
        log::info!("Re-indexing finished.");
    }

    metadata_settings.vector_database_in_use = config.vector_database.provider.to_string();
    app_state
        .databases_layer_entry
        .database
        .update_metadata_settings(metadata_settings)
        .await?;

    Ok(())
}
