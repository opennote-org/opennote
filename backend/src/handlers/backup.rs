use std::collections::HashMap;

use actix_web::{HttpResponse, Result, web};

use crate::{
    api_models::{
        backup::{
            BackupRequest, BackupResponse, GetBackupsListRequest, GetBackupsListResponse,
            RemoveBackupsRequest, RestoreBackupRequest,
        },
        callbacks::GenericResponse,
    },
    app_state::AppState,
    backup::{base::Backup, list_item::BackupListItem},
    database::filters::{get_collections::GetCollectionFilter, get_users::GetUserFilter},
    documents::{
        collection_metadata::CollectionMetadata, document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    },
    identities::user::User,
    tasks_scheduler::TaskStatus,
};

// Sync endpoint
pub async fn remove_backups(
    data: web::Data<AppState>,
    request: web::Json<RemoveBackupsRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    match data
        .backups_storage
        .lock()
        .await
        .remove_backups_by_ids(&request.backup_ids)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to remove backups: {}", e);
            return Ok(
                HttpResponse::InternalServerError().json(GenericResponse::fail(
                    "Failed to remove backups".to_string(),
                    e.to_string(),
                )),
            );
        }
    }

    Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &"")))
}

// Sync endpoint
pub async fn get_backups_list(
    data: web::Data<AppState>,
    request: web::Json<GetBackupsListRequest>,
) -> Result<HttpResponse> {
    let backups: Vec<BackupListItem> = data
        .backups_storage
        .lock()
        .await
        .get_backups_by_scope(&request.0.scope)
        .into_iter()
        .map(|item| item.into())
        .collect();

    Ok(HttpResponse::Ok().json(GenericResponse::succeed(
        "".to_string(),
        &GetBackupsListResponse { backups },
    )))
}

// Async endpoint
pub async fn backup(
    data: web::Data<AppState>,
    request: web::Json<BackupRequest>,
) -> Result<HttpResponse> {
    // TODO: need to distinguish between User scope and others

    // Backup these:
    // 1. User information
    // 2. All resources under this user
    // 3. Database entries that belongs to this user

    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    tokio::spawn(async move {
        let users = match data.database.get_users(&GetUserFilter::default()).await {
            Ok(users) => users,
            Err(e) => {
                log::error!("Failed to fetch users when trying to backup: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(e.to_string()),
                );
                return;
            }
        };

        let user_information_snapshots: Vec<User> = users
            .iter()
            .filter(|item| item.username == request.0.scope.id)
            .map(|item| item.to_owned())
            .collect();

        if user_information_snapshots.is_empty() {
            // Failed to fetch user information when trying to backup, need to use the pre-acquired variables instead
            log::error!(
                "Can't fetch user information when trying to backup: no backup targets found"
            );
            data.tasks_scheduler.lock().await.update_status_by_task_id(
                &task_id,
                TaskStatus::Failed,
                Some("no backup targets found".to_string()),
            );
            return;
        }

        let collection_metadata_snapshots: HashMap<String, CollectionMetadata> = match data
            .database
            .get_collections(&GetCollectionFilter::default(), false)
            .await
        {
            Ok(collections) => collections
                .into_iter()
                .map(|item| (item.id.clone(), item))
                .collect(),
            Err(e) => {
                log::error!("Failed to fetch collections when trying to backup: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(e.to_string()),
                );
                return;
            }
        };

        let document_metadatas: HashMap<String, DocumentMetadata> = collection_metadata_snapshots
            .clone()
            .into_iter()
            .flat_map(|(_, item)| {
                item.documents_metadatas
                    .into_iter()
                    .map(|item| (item.id.clone(), item))
            })
            .collect();

        let document_chunks: Vec<DocumentChunk> = document_metadatas
            .iter()
            .flat_map(|(_, item)| item.chunks.clone())
            .collect();

        let backup: Backup = Backup::new(
            request.0.scope.clone(),
            user_information_snapshots,
            collection_metadata_snapshots,
            document_metadatas,
            document_chunks,
        );
        let backup_id = backup.id.clone();

        match data.backups_storage.lock().await.add_backup(backup).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to save backup: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to save backup: {}", e)),
                );
                return;
            }
        };

        data.tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(BackupResponse { backup_id }).unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}

// Async endpoint
pub async fn restore_backup(
    data: web::Data<AppState>,
    request: web::Json<RestoreBackupRequest>,
) -> Result<HttpResponse> {
    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    tokio::spawn(async move {
        let backup: Backup = match data
            .backups_storage
            .lock()
            .await
            .get_backup_by_id(&request.0.backup_id)
        {
            Some(result) => result,
            None => {
                // Failed to find the backup when trying to restore, need to use the pre-acquired variables instead
                log::error!("Can't find backup when trying to restore: backup not found");
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some("backup not found".to_string()),
                );
                return;
            }
        };

        let users_to_delete: Vec<String> = backup.get_usernames();
        let collection_metadatas_to_delete: Vec<String> = backup.get_collection_metadata_ids();
        let document_metadatas_to_delete: Vec<String> = backup.get_document_metadata_ids();

        // Remove the old data from metadata, user information, database
        match data.database.delete_users(users_to_delete).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to delete users during restore: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to delete users: {}", e)),
                );
                return;
            }
        }

        match data
            .database
            .delete_collections(&collection_metadatas_to_delete)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to delete collections during restore: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to delete collections: {}", e)),
                );
                return;
            }
        }

        match data
            .database
            .delete_documents(&document_metadatas_to_delete)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to delete documents during restore: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to delete documents: {}", e)),
                );
                return;
            }
        }

        match data
            .vector_database
            .delete_documents_from_database(
                &data.config.vector_database,
                &document_metadatas_to_delete,
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!(
                    "Failed to delete old documents from database during restore: {}",
                    e
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!(
                        "Failed to delete old documents from database: {}",
                        e
                    )),
                );
                return;
            }
        }

        // Swap the data from the backup in
        match data
            .database
            .add_users(backup.user_information_snapshots)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to add users during restore: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to add users: {}", e)),
                );
                return;
            }
        }

        match data
            .database
            .add_collections(
                backup
                    .collection_metadata_snapshots
                    .into_iter()
                    .map(|item| item.1)
                    .collect(),
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to add collections during restore: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to add collections: {}", e)),
                );
                return;
            }
        }

        match data
            .database
            .add_documents(
                backup
                    .document_metadata_snapshots
                    .into_iter()
                    .map(|item| item.1)
                    .collect(),
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to add documents during restore: {}", e);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to add documents: {}", e)),
                );
                return;
            }
        }

        match data
            .vector_database
            .add_document_chunks_to_database(
                &data.config.embedder,
                &data.config.vector_database,
                backup.document_chunks_snapshots,
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!(
                    "Failed to add document chunks to database during restore: {}",
                    e
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(format!("Failed to add document chunks to database: {}", e)),
                );
                return;
            }
        }

        data.tasks_scheduler
            .lock()
            .await
            .set_status_to_complete(&task_id, serde_json::to_value("").unwrap());
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}
