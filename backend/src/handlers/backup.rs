use std::collections::HashMap;

use actix_web::{HttpResponse, web};
use anyhow::Result;
use tokio::sync::RwLock;

use crate::{
    api_models::{backup::BackupRequest, callbacks::GenericResponse},
    app_state::AppState,
    configurations::user::UserConfigurations,
    documents::{collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata},
    identities::user::{self, User},
    utilities::acquire_data,
};

// Sync endpoint
pub async fn backup(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<BackupRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, metadata_storage, _, _, user_information_storage) = acquire_data(&data).await;

    // TODO: need to distinguish between User scope and others

    // Backup these:
    // 1. User information
    // 2. All resources under this user
    // 3. Database entries that belongs to this user

    let user_information_snapshots: Vec<User> = user_information_storage
        .lock()
        .await
        .users
        .iter()
        .filter(|item| item.username == request.0.scope.id)
        .map(|item| item.to_owned())
        .collect();

    if user_information_snapshots.is_empty() {
        return Ok(HttpResponse::Ok().json(GenericResponse::fail(
            "".to_string(),
            format!("no backup targets found"),
        )));
    }

    let mut collection_metadata_snapshots: HashMap<String, CollectionMetadata> =
        metadata_storage.lock().await.collections.clone();
    let mut document_metadata_snapshots: HashMap<String, DocumentMetadata> =
        metadata_storage.lock().await.documents.clone();

    for user_information_snapshot in user_information_snapshots {
        let mut collection_metadata_ids: Vec<&String> = Vec::new();
        
        collection_metadata_snapshots = collection_metadata_snapshots
            .into_iter()
            .filter(|(collection_metadata_id, collection_metadata)| {
                let is_contained: bool = user_information_snapshot
                    .resources
                    .contains(collection_metadata_id);

                if is_contained {
                    collection_metadata_ids.push(collection_metadata_id);
                }

                is_contained
            })
            .collect();

        document_metadata_snapshots = document_metadata_snapshots
            .into_iter()
            .filter(|(document_metadata_id, document_metadata)| {
                collection_metadata_ids.contains(&&document_metadata.collection_metadata_id)
            })
            .collect();
    }

    Ok(())
}

// Sync endpoint
pub async fn restore_backup(
    data: web::Data<RwLock<AppState>>,
    request: web::Json,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, user_information_storage) = acquire_data(&data).await;

    match user_information_storage
        .lock()
        .await
        .update_user_configurations(&request.0.username, request.0.user_configurations)
        .await
    {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &"".to_string())))
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string())))
        }
    }
}
