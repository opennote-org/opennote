use std::collections::HashMap;

use actix_web::{HttpResponse, Result, web};
use tokio::sync::RwLock;

use crate::{
    api_models::{
        backup::{
            BackupRequest, BackupResponse, GetBackupsListRequest, GetBackupsListResponse, RemoveBackupsRequest, RestoreBackupRequest
        },
        callbacks::GenericResponse,
    }, app_state::AppState, backup::archieve::{Archieve, ArchieveListItem}, documents::{
        collection_metadata::CollectionMetadata, document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    }, handler_operations::{add_document_chunks_to_database, delete_documents_from_database, get_document_chunks}, identities::user::User, tasks_scheduler::TaskStatus, traits::LoadAndSave, utilities::acquire_data
};

// Sync endpoint
pub async fn remove_backups(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<RemoveBackupsRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, _, archieves_storage) = acquire_data(&data).await;

    match archieves_storage
        .lock()
        .await
    .remove_archieves_by_ids(&request.archieve_ids).await 
    {
        Ok(_) => {},
        Err(e) => {
            log::error!("Failed to remove archives: {}", e);
            return Ok(HttpResponse::InternalServerError().json(GenericResponse::fail(
                "Failed to remove archives".to_string(),
                e.to_string(),
            )));
        }
    }

    Ok(HttpResponse::Ok().json(GenericResponse::succeed(
        "".to_string(),
        &""
    )))
}

// Sync endpoint
pub async fn get_backups_list(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<GetBackupsListRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, _, archieves_storage) = acquire_data(&data).await;

    let archieves: Vec<ArchieveListItem> = archieves_storage
        .lock()
        .await
        .get_archieves_by_scope(&request.0.scope)
        .into_iter()
        .map(|item| item.into())
        .collect();

    Ok(HttpResponse::Ok().json(GenericResponse::succeed(
        "".to_string(),
        &GetBackupsListResponse {
            archieves: archieves,
        },
    )))
}

// Async endpoint
pub async fn backup(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<BackupRequest>,
) -> Result<HttpResponse> {
    // TODO: need to distinguish between User scope and others

    // Backup these:
    // 1. User information
    // 2. All resources under this user
    // 3. Database entries that belongs to this user

    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (
            index_name,
            client,
            metadata_storage,
            tasks_scheduler,
            _,
            user_information_storage,
            archieve_storage,
        ) = acquire_data(&data).await;

        let user_information_snapshots: Vec<User> = user_information_storage
            .lock()
            .await
            .users
            .iter()
            .filter(|item| item.username == request.0.scope.id)
            .map(|item| item.to_owned())
            .collect();

        if user_information_snapshots.is_empty() {
            // Failed to fetch user information when trying to backup, need to use the pre-acquired variables instead
            log::error!(
                "Can't fetch user information when trying to backup: no backup targets found"
            );
            tasks_scheduler.lock().await.update_status_by_task_id(
                &task_id,
                TaskStatus::Failed,
                Some("no backup targets found".to_string()),
            );
            return;
        }

        let mut collection_metadata_snapshots: HashMap<String, CollectionMetadata> =
            metadata_storage.lock().await.collections.clone();
        let mut document_metadata_snapshots: HashMap<String, DocumentMetadata> =
            metadata_storage.lock().await.documents.clone();

        for user_information_snapshot in user_information_snapshots.iter() {
            let mut collection_metadata_ids: Vec<String> = Vec::new();

            collection_metadata_snapshots = collection_metadata_snapshots
                .into_iter()
                .filter(|(collection_metadata_id, _)| {
                    let is_contained: bool = user_information_snapshot
                        .resources
                        .contains(collection_metadata_id);

                    if is_contained {
                        collection_metadata_ids.push(collection_metadata_id.clone());
                    }

                    is_contained
                })
                .collect();

            document_metadata_snapshots = document_metadata_snapshots
                .into_iter()
                .filter(|(_, document_metadata)| {
                    collection_metadata_ids.contains(&&document_metadata.collection_metadata_id)
                })
                .collect();
        }

        // Backup database entries
        let document_chunks_ids: Vec<String> = document_metadata_snapshots
            .iter()
            .flat_map(|(_, document_metadata)| document_metadata.chunks.clone())
            .collect();
        let document_chunks_snapshots: Vec<DocumentChunk> =
            match get_document_chunks(document_chunks_ids, &index_name, &client).await {
                Ok(points) => points,
                Err(e) => {
                    // Failed to get document chunks when trying to backup, need to use the pre-acquired variables instead
                    log::error!("Can't get document chunks when trying to backup: {}", e);
                    tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(e.to_string()),
                    );
                    return;
                }
            };

        let archieve: Archieve = Archieve::new(
            request.0.scope.clone(),
            user_information_snapshots,
            collection_metadata_snapshots,
            document_metadata_snapshots,
            document_chunks_snapshots,
        );
        let archieve_id = archieve.id.clone();

        match archieve_storage.lock().await.add_archieve(archieve).await {
            Ok(_) => {},
            Err(e) => {
                log::error!("Failed to save archive: {}", e);
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to save archive: {}", e)),
                );
                return;
            }
        };

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(BackupResponse { archieve_id }).unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}

// Async endpoint
pub async fn restore_backup(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<RestoreBackupRequest>,
) -> Result<HttpResponse> {
    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (_, client, metadata_storage, tasks_scheduler, config, user_information_storage, archieves_storage) = acquire_data(&data).await;

        let archieve: Archieve = match archieves_storage
            .lock()
            .await
            .get_archieve_by_id(&request.0.archieve_id)
        {
            Some(result) => result,
            None => {
                // Failed to find the archive when trying to restore, need to use the pre-acquired variables instead
                log::error!("Can't find archive when trying to restore: archive not found");
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some("archive not found".to_string()),
                );
                return;
            }
        };

        let users_to_delete: Vec<String> = archieve.get_usernames();
        let collection_metadatas_to_delete: Vec<String> = archieve.get_collection_metadata_ids();
        let document_metadatas_to_delete: Vec<String> = archieve.get_document_metadata_ids();

        // Remove the old data from metadata, user information, database
        user_information_storage.lock()
            .await
            .users
            .retain(|item| !users_to_delete.contains(&item.username));

        {
            let mut metadata_storage = metadata_storage.lock().await;
            metadata_storage.collections
                .retain(|id, _| !collection_metadatas_to_delete.contains(id));
            metadata_storage.documents
                .retain(|id, _| !document_metadatas_to_delete.contains(id));
        }

        match delete_documents_from_database(
            &client,
            &config.database,
            document_metadatas_to_delete,
        ).await
        {
            Ok(_) => {},
            Err(e) => {
                log::error!("Failed to delete old documents from database during restore: {}", e);
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to delete old documents from database: {}", e)),
                );
                return;
            }
        }

        // Swap the data from the backup in
        user_information_storage.lock().await.users.extend(
            archieve.user_information_snapshots
        );

        {
            let mut metadata_storage = metadata_storage.lock().await;
            metadata_storage.collections.extend(
                archieve.collection_metadata_snapshots
            );
            metadata_storage.documents.extend(
                archieve.document_metadata_snapshots
            );
        }

        match add_document_chunks_to_database(
            &client, &config.embedder, &config.database, archieve.document_chunks_snapshots
        ).await
        {
            Ok(_) => {},
            Err(e) => {
                log::error!("Failed to add document chunks to database during restore: {}", e);
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to add document chunks to database: {}", e)),
                );
                return;
            }
        }
        
        match metadata_storage.lock().await.save().await {
            Ok(_) => {},
            Err(e) => {
                log::error!("Failed to save metadata storage during restore: {}", e);
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to save metadata storage: {}", e)),
                );
                return;
            }
        };

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value("").unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}
