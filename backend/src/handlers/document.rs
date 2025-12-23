use std::collections::HashSet;

use actix_web::{
    HttpResponse, Result,
    web::{self, Query},
};
use futures::future::join_all;
use log::{error, info};
use qdrant_client::qdrant::{GetPointsBuilder, PointId};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{
    api_models::{
        callbacks::GenericResponse,
        document::{
            AddDocumentRequest, AddDocumentResponse, DeleteDocumentRequest, DeleteDocumentResponse,
            GetDocumentRequest, GetDocumentsMetadataQuery, UpdateDocumentContentRequest,
            UpdateDocumentMetadataRequest, UpdateDocumentResponse,
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
    documents::{document_chunk::DocumentChunk, document_metadata::DocumentMetadata},
    handler_operations::{
        add_document_chunks_to_database, delete_documents_from_database, preprocess_document,
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
        let (_, db_client, metadata_storage, tasks_scheduler, config, user_information_storage) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match user_information_storage
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

        match add_document_chunks_to_database(
            &db_client,
            metadata_storage,
            metadata,
            &config.embedder,
            &config.database,
            chunks,
        )
        .await
        {
            Ok(_) => {
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
        let (_, db_client, metadata_storage, tasks_scheduler, config, user_information_storage) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match user_information_storage
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
            let request = request.clone();
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
                    store_tasks.push(add_document_chunks_to_database(
                        &db_client,
                        metadata_storage.clone(),
                        metadata,
                        &config.embedder,
                        &config.database,
                        chunks,
                    ));
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
        let (_, db_client, metadata_storage, tasks_scheduler, config, _) =
            acquire_data(&data).await;

        match delete_documents_from_database(
            &db_client,
            &mut metadata_storage.lock().await,
            &config.database,
            vec![request.document_metadata_id.clone()],
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
    request: web::Json<UpdateDocumentMetadataRequest>,
) -> Result<HttpResponse> {
    let (_, _, metadata_storage, _, _, _) = acquire_data(&data).await;

    match metadata_storage
        .lock()
        .await
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
        let (_, db_client, metadata_storage, tasks_scheduler, config, user_information_storage) =
            acquire_data(&data).await;

        let user_configurations: UserConfigurations = match user_information_storage
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

        match delete_documents_from_database(
            &db_client,
            &mut metadata_storage.lock().await,
            &config.database,
            vec![request.document_metadata_id.clone()],
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

        let mut metadata: DocumentMetadata = DocumentMetadata::new(
            request.title.clone(),
            request.collection_metadata_id.clone(),
        );
        let metdata_id: String = metadata.metadata_id.clone();

        let chunks: Vec<DocumentChunk> = DocumentChunk::slice_document_by_period(
            &request.content,
            user_configurations.search.document_chunk_size,
            &metadata.metadata_id,
            &metadata.collection_metadata_id,
        );

        metadata.chunks = chunks.iter().map(|chunk| chunk.id.clone()).collect();

        match add_document_chunks_to_database(
            &db_client,
            metadata_storage.clone(),
            metadata,
            &config.embedder,
            &config.database,
            chunks,
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
    query: Query<GetDocumentsMetadataQuery>,
) -> Result<HttpResponse> {
    let (_, _, metadata_storage, _, _, _) = acquire_data(&data).await;

    let metadata: Vec<DocumentMetadata> = metadata_storage
        .lock()
        .await
        .documents
        .iter()
        .filter(|(_, document_metadata)| {
            document_metadata.collection_metadata_id == query.0.collection_metadata_id
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
    let (index_name, db_client, metadata_storage, _, _, _) = acquire_data(&data).await;

    // Acquire chunk ids
    let mut acquired_chunks: Vec<DocumentChunk> = Vec::new();
    if let Some(document_metadata) = metadata_storage
        .lock()
        .await
        .documents
        .get(&request.document_metadata_id)
    {
        match db_client
            .get_points(
                GetPointsBuilder::new(
                    index_name,
                    document_metadata
                        .chunks
                        .clone()
                        .into_iter()
                        .map(|chunk| chunk.into())
                        .collect::<Vec<PointId>>(),
                )
                .with_payload(true),
            )
            .await
        {
            Ok(result) => {
                acquired_chunks = result
                    .result
                    .into_iter()
                    .map(|point| point.into())
                    .collect();
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
