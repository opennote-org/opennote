use std::collections::HashSet;

use actix_web::{HttpResponse, Result, web};
use futures::future::join_all;
use log::{error, info};
use serde_json::json;

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
    databases::database::filters::{get_documents::GetDocumentFilter, get_users::GetUserFilter},
    documents::{
        document_chunk::DocumentChunk, document_metadata::DocumentMetadata,
        operations::preprocess_document,
    },
    embedder::vectorize,
    tasks_scheduler::TaskStatus,
};

pub async fn add_document(
    data: web::Data<AppState>,
    request: web::Json<AddDocumentRequest>,
) -> Result<HttpResponse> {
    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        let user_configurations: UserConfigurations = match data
            .database
            .get_users(&GetUserFilter {
                usernames: vec![request.0.username.clone()],
                ..Default::default()
            })
            .await
        {
            Ok(mut result) => {
                if let Some(user) = result.pop() {
                    user.configuration
                } else {
                    let message = format!("User {} not found", request.0.username);
                    error!("{}", message);
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(message),
                    );
                    return;
                }
            }
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        let (mut metadata, chunks, metadata_id) = preprocess_document(
            &request.title,
            &request.content,
            &request.collection_metadata_id,
            user_configurations.search.document_chunk_size,
        );

        match vectorize(&data.config.embedder, chunks).await {
            Ok(chunks) => metadata.chunks = chunks,
            Err(error) => {
                error!("Failed to vectorize document chunks: {}", error);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        match data
            .databases_layer_entry
            .add_documents(
                &data.config.embedder,
                &data.config.vector_database,
                vec![metadata],
            )
            .await
        {
            Ok(_) => {}
            Err(error) => {
                error!("Failed to add document: {}", error);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        data.tasks_scheduler.lock().await.set_status_to_complete(
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
    data: web::Data<AppState>,
    request: web::Json<ImportDocumentsRequest>,
) -> Result<HttpResponse> {
    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        let user_configurations: UserConfigurations = match data
            .database
            .get_users(&GetUserFilter {
                usernames: vec![request.0.username.clone()],
                ..Default::default()
            })
            .await
        {
            Ok(mut result) => {
                if let Some(user) = result.pop() {
                    user.configuration
                } else {
                    let message = format!("User {} not found", request.0.username);
                    error!("{}", message);
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(message),
                    );
                    return;
                }
            }
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
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
        let mut document_metadatas = Vec::new();
        let mut document_metadata_ids = Vec::new();
        for (index, task) in preprocess_tasks.into_iter().enumerate() {
            match task.await {
                Ok((mut metadata, chunks, document_metadata_id)) => {
                    metadata.chunks = chunks;
                    document_metadatas.push(metadata);
                    document_metadata_ids.push(document_metadata_id);
                }
                Err(err) => {
                    error!("Failed to preprocess: {}", err);
                    failures.insert(request.0.imports[index].clone());
                    continue;
                }
            }
        }

        match data
            .databases_layer_entry
            .add_documents(
                &data.config.embedder,
                &data.config.vector_database,
                document_metadatas,
            )
            .await
        {
            Ok(_) => {}
            Err(error) => {
                error!("Failed to add imported documents: {}", error);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        if !failures.is_empty() {
            error!("Failed importing {} documents", failures.len());

            // Prevent failing a whole task with multiple import requests
            if request.0.imports.len() == 1 {
                data.tasks_scheduler.lock().await.update_status_by_task_id(
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

        data.tasks_scheduler.lock().await.set_status_to_complete(
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
    data: web::Data<AppState>,
    request: web::Json<DeleteDocumentRequest>,
) -> Result<HttpResponse> {
    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        match data
            .databases_layer_entry
            .delete_documents(
                &data.config.vector_database,
                &vec![request.0.document_metadata_id.clone()],
            )
            .await
        {
            Ok(_) => {}
            Err(error) => {
                error!("Failed to delete document: {}", error);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        data.tasks_scheduler.lock().await.set_status_to_complete(
            &task_id,
            serde_json::to_value(DeleteDocumentResponse {
                document_metadata_id: request.document_metadata_id,
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
    data: web::Data<AppState>,
    request: web::Json<UpdateDocumentMetadataRequest>,
) -> Result<HttpResponse> {
    match data
        .database
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
    data: web::Data<AppState>,
    request: web::Json<UpdateDocumentContentRequest>,
) -> Result<HttpResponse> {
    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        let user_configurations: UserConfigurations = match data
            .database
            .get_users(&GetUserFilter {
                usernames: vec![request.0.username.clone()],
                ..Default::default()
            })
            .await
        {
            Ok(mut result) => {
                if let Some(user) = result.pop() {
                    user.configuration
                } else {
                    let message = format!("User {} not found", request.0.username);
                    error!("{}", message);
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(message),
                    );
                    return;
                }
            }
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
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
            let mut metadata = match data
                .database
                .get_documents(&GetDocumentFilter {
                    ids: vec![request.document_metadata_id.clone()],
                    ..Default::default()
                })
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    let message: String = format!(
                        "Document {} deletion failed due to {}",
                        &request.document_metadata_id, error,
                    );
                    log::warn!("{}", message);
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(message),
                    );
                    return;
                }
            };

            if let Some(metadata) = metadata.pop() {
                metadata
            } else {
                let message = format!("Document {} not found", &request.document_metadata_id);
                error!("{}", message);
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(message),
                );
                return;
            }
        };

        let metdata_id: String = metadata.id.clone();

        let chunks: Vec<DocumentChunk> = DocumentChunk::slice_document_automatically(
            &request.content,
            user_configurations.search.document_chunk_size,
            &metadata.id,
            &metadata.collection_metadata_id,
        );

        metadata.chunks = chunks.clone();

        data.databases_layer_entry
            .update_documents(
                &data.config.embedder,
                &data.config.vector_database,
                vec![metadata],
            )
            .await;

        data.tasks_scheduler.lock().await.set_status_to_complete(
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
    data: web::Data<AppState>,
    query: web::Json<GetDocumentsMetadataQuery>,
) -> Result<HttpResponse> {
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

    let metadata: Vec<DocumentMetadata> =
        if let Some(ref document_metadata_ids) = query.document_metadata_ids {
            match data
                .database
                .get_documents(&GetDocumentFilter {
                    ids: document_metadata_ids.clone(),
                    ..Default::default()
                })
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    error!("Failed to get documents metadata: {}", error);
                    return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                        "".to_string(),
                        format!("Failed to get documents metadata: {}", error),
                    )));
                }
            }
        } else if let Some(ref collection_metadata_id) = query.collection_metadata_id {
            match data
                .database
                .get_documents(&GetDocumentFilter {
                    collection_metadata_ids: vec![collection_metadata_id.clone()],
                    ..Default::default()
                })
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    error!("Failed to get documents metadata: {}", error);
                    return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                        "".to_string(),
                        format!("Failed to get documents metadata: {}", error),
                    )));
                }
            }
        } else {
            Vec::new()
        };

    Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &metadata)))
}

// Sync endpoint
pub async fn get_document_content(
    data: web::Data<AppState>,
    request: web::Json<GetDocumentRequest>,
) -> Result<HttpResponse> {
    let document_metadatas = match data
        .database
        .get_documents(&GetDocumentFilter {
            ids: vec![request.document_metadata_id.clone()],
            ..Default::default()
        })
        .await
    {
        Ok(result) => result,
        Err(error) => {
            error!("Failed to get documents metadata: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                format!("Failed to get documents metadata: {}", error),
            )));
        }
    };

    let acquired_chunks: Vec<DocumentChunk> = document_metadatas
        .into_iter()
        .flat_map(|item| item.chunks)
        .collect();

    Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &acquired_chunks)))
}

/// For now, we only allow re-indexing a user's documents.
/// To re-index all documents regardless of ownerships, it needs to re-configure the embedding model
/// in the configurations json, then restart the backend.
pub async fn reindex(
    data: web::Data<AppState>,
    request: web::Json<ReindexRequest>,
) -> Result<HttpResponse> {
    let task_id = data.tasks_scheduler.lock().await.create_new_task();
    let task_id_cloned = task_id.clone();

    // Perform operations asynchronously
    tokio::spawn(async move {
        let user_configurations: UserConfigurations = match data
            .database
            .get_users(&GetUserFilter {
                usernames: vec![request.0.username.clone()],
                ..Default::default()
            })
            .await
        {
            Ok(mut result) => {
                if let Some(user) = result.pop() {
                    user.configuration
                } else {
                    let message = format!("User {} not found", request.0.username);
                    error!("{}", message);
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(message),
                    );
                    return;
                }
            }
            Err(error) => {
                // Failed to write the task status back to the scheduler, need to use the pre-acquired variables instead
                error!(
                    "Can't fetch user configurations when trying adding a document: {}",
                    error
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        let resource_ids: Vec<String> = match data
            .database
            .get_resource_ids_by_username(&request.0.username)
            .await
        {
            Ok(ids) => ids,
            Err(error) => {
                error!(
                    "Failed to get resource IDs for user {}: {}",
                    request.0.username, error
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        };

        // 1. Clean up existing DocumentMetadata
        // 2. Get the document contents
        // 3. Re-slice the document contents, then put the chunk ids to corresponding DocumentMetadata

        // Get all documents by their collections

        // TaskBatch: (collection metadata id, DocumentMetadata, content)

        // Get the metadata first.
        // We will need to use concurrency in fetching document contents to maximize efficiency.
        let mut get_document_contents_tasks_data = Vec::new();

        // Reserved for deleting them from the database
        let mut metadata_ids_to_delete: Vec<String> = Vec::new();

        for collection_metadata_id in resource_ids {
            let document_metadatas = match data
                .database
                .get_documents(&GetDocumentFilter {
                    collection_metadata_ids: vec![collection_metadata_id.clone()],
                    ..Default::default()
                })
                .await
            {
                Ok(ids) => ids,
                Err(error) => {
                    error!(
                        "Failed to get document metadata IDs for collection {}: {}",
                        collection_metadata_id, error
                    );
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
                        &task_id,
                        TaskStatus::Failed,
                        Some(error.to_string()),
                    );
                    return;
                }
            };

            metadata_ids_to_delete.extend(
                document_metadatas
                    .iter()
                    .map(|item| item.id.clone())
                    .collect::<Vec<String>>(),
            );

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
                    let mut content: String = String::new();
                    for chunk in document_metadata.chunks.iter() {
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

                document_metadata.chunks = chunks.clone();

                (document_metadata, chunks)
            }));
        }

        // Remove old chunks from the database before updating the new ones to prevent conflicts.

        match data
            .vector_database
            .delete_documents_from_database(&data.config.vector_database, &metadata_ids_to_delete)
            .await
        {
            Ok(_) => {}
            Err(_) => {
                data.tasks_scheduler.lock().await.update_status_by_task_id(
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
                    final_update_tasks.push(data.vector_database.add_document_chunks_to_database(
                        &data.config.embedder,
                        &data.config.vector_database,
                        document_chunks,
                    ));

                    metadatas_to_update.push(document_metadata);
                }
                Err(error) => {
                    error!(
                        "Failed to re-index the user {} collections: {}",
                        &request.0.username, error
                    );
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
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
                    data.tasks_scheduler.lock().await.update_status_by_task_id(
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
        match data.database.update_documents(metadatas_to_update).await {
            Ok(_) => {}
            Err(error) => {
                error!(
                    "Failed to re-index the user {} collections: {}",
                    &request.0.username, error
                );
                data.tasks_scheduler.lock().await.update_status_by_task_id(
                    &task_id,
                    TaskStatus::Failed,
                    Some(error.to_string()),
                );
                return;
            }
        }

        data.tasks_scheduler.lock().await.set_status_to_complete(
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
