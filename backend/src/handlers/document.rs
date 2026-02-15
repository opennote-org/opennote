use std::collections::HashSet;

use actix_web::{HttpResponse, Result, web};
use futures::future::join_all;
use log::{error, info};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{
    api_models::{
        callbacks::GenericResponse,
        document::{
            AddDocumentRequest, AddDocumentResponse, DeleteDocumentRequest, DeleteDocumentResponse,
            GetDocumentRequest, GetDocumentsMetadataQuery, ReindexRequest, ReindexResponse,
            UpdateDocumentContentRequest, UpdateDocumentMetadataRequest, UpdateDocumentResponse,
        },
    },
    app_state::AppState,
    configurations::user::UserConfigurations,
    connectors::{
        models::ImportTaskIntermediate,
        relationship_database::RelationshipDatabaseConnector,
        requests::{ImportDocumentsRequest, ImportTask, ImportType},
        responses::ImportDocumentsResponse,
        text_file::TextFileConnector,
        traits::Connector,
        webpage::WebpageConnector,
    },
    documents::{
        document_chunk::DocumentChunk, document_metadata::DocumentMetadata,
        operations::preprocess_document,
    },
    tasks_scheduler::TaskStatus,
    utilities::acquire_data,
};

pub async fn add_document(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<AddDocumentRequest>,
) -> Result<HttpResponse> {
    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (vector_database, metadata_storage, tasks_scheduler, config, identities_storage, _) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match identities_storage
            .lock()
            .await
            .get_user_configurations(&request.0.username)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        let (metadata, chunks, metadata_id) = preprocess_document(
            &request.title,
            &request.content,
            &request.collection_metadata_id,
            user_configurations.search.document_chunk_size,
        );

        match vector_database
            .add_document_chunks_to_database(&config.embedder, &config.vector_database, chunks)
            .await
        {
            Ok(_) => {
                match metadata_storage.lock().await.add_document(metadata).await {
                    Ok(_) => {}
                    Err(error) => {
                        error!("Failed to update document metadata: {}", error);
                        tasks_scheduler.lock().await.update_status_by_task_id(
                            &task_id,
                            TaskStatus::Failed,
                            Some(error.to_string()),
                        );
                        return;
                    }
                }
                info!("Task {} has finished adding documents.", task_id);
            }
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!("Failed when trying saving a document: {}", error);
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(AddDocumentResponse {
                document_metadata_id: metadata_id.clone(),
            })
            .unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}

pub async fn import_documents(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<ImportDocumentsRequest>,
) -> Result<HttpResponse> {
    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (vector_database, metadata_storage, tasks_scheduler, config, identities_storage, _) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match identities_storage
            .lock()
            .await
            .get_user_configurations(&request.0.username)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        // Select a connector
        let mut import_tasks = Vec::new();
        for import_task in request.0.imports.iter() {
            import_tasks.push(match import_task.import_type {
                ImportType::TextFile => {
                    TextFileConnector::get_intermediate(import_task.artifact.clone())
                }
                ImportType::Webpage => {
                    WebpageConnector::get_intermediate(import_task.artifact.clone())
                }
                ImportType::RelationshipDatabase => {
                    RelationshipDatabaseConnector::get_intermediate(import_task.artifact.clone())
                }
            });
        }

        // Get intermediates
        let results = join_all(import_tasks).await;
        let mut preprocess_tasks = Vec::new();
        let mut failures: HashSet<ImportTask> = HashSet::new();
        for (index, result) in results.into_iter().enumerate() {
            let result: ImportTaskIntermediate = match result {
                Ok(intermediate) => intermediate,
                Err(err) => {
                    error!("Failed to get intermediate: {}", err);
                    failures.insert(request.0.imports[index].clone());
                    continue;
                }
            };
            let request: ImportDocumentsRequest = request.clone();
            preprocess_tasks.push(tokio::spawn(async move {
                preprocess_document(
                    &result.title,
                    &result.content,
                    &request.collection_metadata_id,
                    user_configurations.search.document_chunk_size,
                )
            }));
        }

        // Preprocess the intermediates
        let mut store_tasks = Vec::new();
        for (index, task) in preprocess_tasks.into_iter().enumerate() {
            match task.await {
                Ok((metadata, chunks, _)) => {
                    store_tasks.push({
                        vector_database.add_document_chunks_to_database_and_metadata_storage(
                            &config.embedder,
                            &config.vector_database,
                            chunks,
                            metadata_storage.clone(),
                            metadata,
                        )
                    });
                }
                Err(err) => {
                    error!("Failed to preprocess: {}", err);
                    failures.insert(request.0.imports[index].clone());
                    continue;
                }
            }
        }

        let store_results = join_all(store_tasks).await;
        let mut document_metadata_ids = Vec::new();

        for (index, store_result) in store_results.into_iter().enumerate() {
            match store_result {
                Ok(result) => {
                    info!(
                        "Task {} has finished importing document id {}.",
                        task_id, result
                    );
                    document_metadata_ids.push(result);
                }
                Err(err) => {
                    error!("Failed to store an imported document: {}", err);
                    failures.insert(request.0.imports[index].clone());
                    continue;
                }
            }
        }

        if !failures.is_empty() {
            error!("Failed importing {} documents", failures.len());

            // Prevent failing a whole task with multiple import requests
            if request.0.imports.len() == 1 {
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(
                        format!(
                            "{} documents failed to get uploaded. You may need to lower the chunk size, or switch to a model that has a larger context window. Or, you may need to check the backend log for details.",
                            failures.len()
                        )
                    ),
                );
                return;
            }
        }

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(ImportDocumentsResponse {
                failed_import_tasks: failures.into_iter().map(|item| item).collect(),
                document_metadata_ids,
            })
            .unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}

pub async fn delete_document(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<DeleteDocumentRequest>,
) -> Result<HttpResponse> {
    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (vector_database, metadata_storage, tasks_scheduler, config, _, _) =
            acquire_data(&data).await;

        let mut metadata_storage = metadata_storage.lock().await;
        match metadata_storage
            .remove_document(&request.document_metadata_id)
            .await
        {
            Some(_) => {}
            None => {
                let message: String = format!(
                    "Document {} was not found when trying to delete",
                    &request.document_metadata_id
                );
                log::warn!("{}", message);
            }
        };

        match vector_database
            .delete_documents_from_database(
                &config.vector_database,
                &vec![request.document_metadata_id.clone()],
            )
            .await
        {
            Ok(_) => {}
            Err(_) => {
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    None,
                );
                return;
            }
        }

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(DeleteDocumentResponse {
                document_metadata_id: request.document_metadata_id.clone(),
            })
            .unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}

/// Sync endpoint
pub async fn update_documents_metadata(
    data: web::Data<RwLock<AppState>>,
    mut request: web::Json<UpdateDocumentMetadataRequest>,
) -> Result<HttpResponse> {
    let (_, metadata_storage, _, _, _, _) = acquire_data(&data).await;

    let mut metadata_storage = metadata_storage.lock().await;

    match metadata_storage
        .verify_immutable_fields_in_document_metadatas(&mut request.0.document_metadatas)
        .await
    {
        Ok(_) => {}
        Err(error) => {
            error!(
                "Failed to verify immutable fields in document metadatas: {}",
                error
            );
            return Ok(
                HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string()))
            );
        }
    };

    match metadata_storage
        .update_documents(request.0.document_metadatas)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &json!({})))),
        Err(error) => {
            error!("Failed when trying updating documents metadata: {}", error);
            return Ok(
                HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string()))
            );
        }
    }
}

pub async fn update_document_content(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<UpdateDocumentContentRequest>,
) -> Result<HttpResponse> {
    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (vector_database, metadata_storage, tasks_scheduler, config, identities_storage, _) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match identities_storage
            .lock()
            .await
            .get_user_configurations(&request.0.username)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        // Isolate the access to the locked metadata storage to prevent potential deadlocking
        // in the following code.
        //
        // We modify the old metadata after done uploading new chunks to the database to
        // prevent accidentally creating new docs.
        let mut metadata: DocumentMetadata = {
            let metadata_storage = metadata_storage.lock().await;
            let metadata = match metadata_storage
                .get_document(&request.document_metadata_id)
                .await
            {
                Some(result) => result.to_owned(),
                None => {
                    let message: String = format!(
                        "Document {} was not found when trying to delete",
                        &request.document_metadata_id
                    );
                    log::warn!("{}", message);
                    tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(message),
                    );
                    return;
                }
            };

            match vector_database
                .delete_documents_from_database(
                    &config.vector_database,
                    &vec![request.document_metadata_id.clone()],
                )
                .await
            {
                Ok(_) => {}
                Err(error) => {
                    tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(error.to_string()),
                    );
                    return;
                }
            }

            metadata
        };

        let metdata_id: String = metadata.id.clone();

        let chunks: Vec<DocumentChunk> = DocumentChunk::slice_document_automatically(
            &request.content,
            user_configurations.search.document_chunk_size,
            &metadata.id,
            &metadata.collection_metadata_id,
        );

        metadata.chunks = chunks.iter().map(|chunk| chunk.id.clone()).collect();

        match vector_database
            .add_document_chunks_to_database(&config.embedder, &config.vector_database, chunks)
            .await
        {
            Ok(_) => {
                match metadata_storage
                    .lock()
                    .await
                    .update_documents(vec![metadata])
                    .await
                {
                    Ok(_) => {}
                    Err(error) => {
                        error!("Failed to update document metadata: {}", error);
                        tasks_scheduler.lock().await.update_status_by_task_id(
                            &task_id,
                            TaskStatus::Failed,
                            Some(error.to_string()),
                        );
                        return;
                    }
                }
                info!("Task {} has finished updating documents.", task_id);
            }
            Err(error) => {
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(UpdateDocumentResponse {
                document_metadata_id: metdata_id,
            })
            .unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}

// Sync endpoint
pub async fn get_documents_metadata(
    data: web::Data<RwLock<AppState>>,
    query: web::Json<GetDocumentsMetadataQuery>,
) -> Result<HttpResponse> {
    let (_, metadata_storage, _, _, _, _) = acquire_data(&data).await;

    let is_query_not_valid: bool = query.0.collection_metadata_id.is_some()
        == query.0.document_metadata_ids.is_some()
        || query.0.collection_metadata_id.is_none() == query.0.document_metadata_ids.is_none();

    if is_query_not_valid {
        error!(
            "Wrong query received when trying to get documents metadata: {:?}",
            &query.0
        );
        return Ok(HttpResponse::Ok().json(GenericResponse::fail(
            "".to_string(),
            format!("You should either supply the collection metadata id or the document metadata ids, not both"),
        )));
    }

    let metadata: Vec<DocumentMetadata> = metadata_storage
        .lock()
        .await
        .documents
        .iter()
        .filter(|(_, document_metadata)| {
            match &query.0.collection_metadata_id {
                Some(result) => {
                    return document_metadata.collection_metadata_id == *result;
                }
                None => {}
            }

            match &query.0.document_metadata_ids {
                Some(result) => {
                    return result.contains(&document_metadata.id);
                }
                None => {}
            }

            false
        })
        .map(|(_, document_metadata)| document_metadata.to_owned())
        .collect();

    Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &metadata)))
}

// Sync endpoint
pub async fn get_document_content(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<GetDocumentRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (vector_database, metadata_storage, _, _, _, _) = acquire_data(&data).await;

    // Acquire chunk ids
    let mut acquired_chunks: Vec<DocumentChunk> = Vec::new();
    if let Some(document_metadata) = metadata_storage
        .lock()
        .await
        .documents
        .get(&request.document_metadata_id)
    {
        match vector_database
            .get_document_chunks(document_metadata.chunks.clone())
            .await
        {
            Ok(result) => {
                acquired_chunks = result;
            }
            Err(error) => {
                error!(
                    "Failed when trying getting document from the database {}: {}",
                    &request.document_metadata_id, error
                );
                return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                    "".to_string(),
                    format!("Failed to get the document"),
                )));
            }
        }
    }

    Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &acquired_chunks)))
}

/// For now, we only allow re-indexing a user's documents.
/// To re-index all documents regardless of ownerships, it needs to re-configure the embedding model
/// in the configurations json, then restart the backend.
pub async fn reindex(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<ReindexRequest>,
) -> Result<HttpResponse> {
    let task_id = data
        .write()
        .await
        .tasks_scheduler
        .lock()
        .await
        .create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        // Pull what we need out of AppState without holding the lock during I/O
        let (vector_database, metadata_storage, tasks_scheduler, config, identities_storage, _) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match identities_storage
            .lock()
            .await
            .get_user_configurations(&request.0.username)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        let resource_ids: Vec<String> = identities_storage
            .lock()
            .await
            .get_resource_ids_by_username(&request.0.username)
            .iter()
            .map(|item| item.to_owned().to_owned())
            .collect();

        // 1. Clean up existing DocumentMetadata
        // 2. Get the document contents
        // 3. Re-slice the document contents, then put the chunk ids to corresponding DocumentMetadata

        // Get all documents by their collections

        // TaskBatch: (collection metadata id, DocumentMetadata, content)

        // Get the metadata first.
        // We will need to use concurrency in fetching document contents to maximize efficiency.
        let mut metadata_storage = metadata_storage.lock().await;
        let mut get_document_contents_tasks_data = Vec::new();

        // Reserved for deleting them from the database
        let mut metadata_ids_to_delete: Vec<String> = Vec::new();

        for collection_metadata_id in resource_ids {
            let document_metadata_ids =
                metadata_storage.get_document_ids_by_collection(&collection_metadata_id);

            metadata_ids_to_delete.extend(
                document_metadata_ids
                    .iter()
                    .map(|item| item.to_owned().to_owned())
                    .collect::<Vec<String>>(),
            );

            let mut document_metadatas = Vec::new();

            for document_metadata_id in document_metadata_ids {
                if let Some(document_metadata) =
                    metadata_storage.documents.get(document_metadata_id)
                {
                    document_metadatas.push(document_metadata.to_owned());
                }
            }

            get_document_contents_tasks_data.push((collection_metadata_id, document_metadatas));
        }

        // 2. Get the document contents
        // There is no mutation to the document metadata chunks ids yet.
        // We will save that for the final updating phase to avoid losing data when failing getting document chunks.
        let mut get_document_contents_tasks = Vec::new();
        for (collection_metadata_id, document_metadatas) in get_document_contents_tasks_data {
            for document_metadata in document_metadatas {
                let collection_metadata_id: String = collection_metadata_id.clone();
                get_document_contents_tasks.push(async {
                    let chunks: Vec<DocumentChunk> = vector_database
                        .get_document_chunks(document_metadata.chunks.clone())
                        .await
                        .unwrap_or(vec![]);

                    let mut content: String = String::new();
                    for chunk in chunks {
                        content.push_str(&chunk.content);
                    }

                    (collection_metadata_id, document_metadata, content)
                });
            }
        }

        // 3. Re-slice the document contents, then put the chunk ids to corresponding DocumentMetadata
        let results = join_all(get_document_contents_tasks).await;
        let mut slicing_tasks = Vec::new();
        for (collection_metadata_id, mut document_metadata, document_content) in results {
            slicing_tasks.push(tokio::spawn(async move {
                // Concurrently update the document chunks and their DocumentMetadata
                let metadata_id: String = document_metadata.id.clone();

                let chunks: Vec<DocumentChunk> = DocumentChunk::slice_document_automatically(
                    &document_content,
                    user_configurations.search.document_chunk_size,
                    &metadata_id,
                    &collection_metadata_id,
                );

                document_metadata.chunks = chunks.iter().map(|chunk| chunk.id.clone()).collect();

                (document_metadata, chunks)
            }));
        }

        // Remove old chunks from the database before updating the new ones to prevent conflicts.

        match vector_database
            .delete_documents_from_database(&config.vector_database, &metadata_ids_to_delete)
            .await
        {
            Ok(_) => {}
            Err(_) => {
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    None,
                );
                return;
            }
        }

        let mut final_update_tasks = Vec::new();
        let mut metadatas_to_update = Vec::new();
        for task in slicing_tasks {
            match task.await {
                Ok((document_metadata, document_chunks)) => {
                    final_update_tasks.push(vector_database.add_document_chunks_to_database(
                        &config.embedder,
                        &config.vector_database,
                        document_chunks,
                    ));

                    metadatas_to_update.push(document_metadata);
                }
                Err(error) => {
                    error!(
                        "Failed to re-index the user {} collections: {}",
                        &request.0.username, error
                    );
                    tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(error.to_string()),
                    );
                    return;
                }
            }
        }

        let results = join_all(final_update_tasks).await;
        for result in results {
            match result {
                Ok(_) => {}
                Err(error) => {
                    error!(
                        "Failed to re-index the user {} collections: {}",
                        &request.0.username, error
                    );
                    tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(error.to_string()),
                    );
                    return;
                }
            }
        }

        // For returning in the response
        let metadatas_count: usize = metadatas_to_update.len();

        // Finally, update the metadata
        match metadata_storage
            .update_documents_with_new_chunks(metadatas_to_update)
            .await
        {
            Ok(_) => {}
            Err(error) => {
                error!(
                    "Failed to re-index the user {} collections: {}",
                    &request.0.username, error
                );
                tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(ReindexResponse {
                documents_reindexed: metadatas_count,
            })
            .unwrap(),
        );
    });

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::in_progress(task_id_cloned))
        .into())
}
