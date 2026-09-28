use std::sync::RwLock;

use actix_web::{
    HttpResponse,
    web::{Bytes, Data},
};
use anyhow::{Result, anyhow};
use serde::de::DeserializeOwned;

use opennote_models::{
    configurations::{fields::EmbedderConfig, server::ServerConfigurations},
    server::{
        requests::decrypt_request,
        responses::{create_bad_response, create_error_response},
    },
    traits::{CompareNecessaryChanges, GetSystemConfigurations},
};

use crate::reindex::ReindexSessionManager;

/// Check if server and desktop configurations mismatch
pub fn validate_incoming_embedder_configurations(
    server_embedder: &EmbedderConfig,
    desktop_embedder: &EmbedderConfig,
) -> Result<()> {
    // Prompt the desktop to sync the configurations, if anything mismatches
    if desktop_embedder.compare_necessary_changes(server_embedder) {
        return Err(anyhow!(
            "Desktop embedder had changed. Both sides must stay the same"
        ));
    }

    Ok(())
}

/// Check request encryption, embedder configurations and reindex status
///
/// if reindex_session_manager is not None, it will check whether the reindex is ongoing.
pub fn perform_pre_request_validations<T>(
    configurations: &tokio::sync::MutexGuard<'_, ServerConfigurations>,
    reindex_session_manager: Option<Data<RwLock<ReindexSessionManager>>>,
    request: Bytes,
) -> Result<T, HttpResponse>
where
    T: DeserializeOwned + GetSystemConfigurations,
{
    let request: T = match decrypt_request(request, &configurations.shared_key) {
        Ok(req) => req,
        Err(e) => {
            return Err(create_bad_response(format!(
                "Failed to decrypt request: {}",
                e
            )));
        }
    };

    match validate_incoming_embedder_configurations(
        &configurations.system.embedder,
        &request.get_system_configurations().embedder,
    ) {
        Ok(()) => {}
        Err(e) => {
            return Err(create_error_response(
                e.to_string(),
                &configurations.shared_key,
            ));
        }
    }

    // Write sessions won't be performed if there is a reindex operation ongoing
    if let Some(reindex_session_manager) = reindex_session_manager {
        let reindex_session_manager = reindex_session_manager.read().unwrap();
        if reindex_session_manager.get_session().is_some() {
            return Err(create_error_response(
                "A reindex session is ongoing. Write operations has been suspended by the server"
                    .to_string(),
                &configurations.shared_key,
            ));
        }
    }

    Ok(request)
}
