use actix_web::{HttpResponse, Result, web};
use log::error;
use tokio::sync::RwLock;

use crate::{
    api_models::{callbacks::GenericResponse, search::SearchDocumentRequest},
    app_state::AppState,
    documents::{
        document_chunk::DocumentChunkSearchResult, operations::retrieve_document_ids_by_scope,
    },
    search::{
        build_search_results, keyword::search_documents, semantic::search_documents_semantically,
    },
    utilities::acquire_data,
};

// Sync endpoint
pub async fn intelligent_search(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<SearchDocumentRequest>,
) -> Result<HttpResponse> {
    // Perform operations synchronously
    // Pull what we need out of AppState without holding the lock during I/O
    let (index_name, db_client, metadata_storage, _, config, identities_storage, _) =
        acquire_data(&data).await;

    let document_metadata_ids: Vec<String> = retrieve_document_ids_by_scope(
        &mut metadata_storage.lock().await,
        &mut identities_storage.lock().await,
        request.0.scope.search_scope,
        &request.0.scope.id,
    );

    match search_documents_semantically(
        &db_client,
        document_metadata_ids,
        &index_name,
        &request.0.query,
        request.0.top_n,
        &config.embedder.provider,
        &config.embedder.base_url,
        &config.embedder.api_key,
        &config.embedder.model,
        &config.embedder.encoding_format,
    )
    .await
    {
        Ok(results) => {
            let metadata_storage = metadata_storage.lock().await;
            let results: Vec<DocumentChunkSearchResult> = build_search_results(
                Some(results),
                None,
                &metadata_storage.collections,
                &metadata_storage.documents,
            );

            return Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &results)));
        }
        Err(error) => {
            error!("Failed when trying searching: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Failed to talk to the database. Please check the connection.".to_string(),
            )));
        }
    };
}

// Sync endpoint
pub async fn search(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<SearchDocumentRequest>,
) -> Result<HttpResponse> {
    // Perform operations synchronously
    // Pull what we need out of AppState without holding the lock during I/O
    let (index_name, db_client, metadata_storage, _, _, identities_storage, _) =
        acquire_data(&data).await;

    let document_metadata_ids: Vec<String> = retrieve_document_ids_by_scope(
        &mut metadata_storage.lock().await,
        &mut identities_storage.lock().await,
        request.0.scope.search_scope,
        &request.0.scope.id,
    );

    match search_documents(
        &db_client,
        document_metadata_ids,
        &index_name,
        &request.0.query,
        request.0.top_n,
    )
    .await
    {
        Ok(results) => {
            let metadata_storage = metadata_storage.lock().await;
            let results: Vec<DocumentChunkSearchResult> = build_search_results(
                None,
                Some(results),
                &metadata_storage.collections,
                &metadata_storage.documents,
            );

            return Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &results)));
        }
        Err(error) => {
            error!("Failed when trying searching: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Failed to talk to the database. Please check the connection.".to_string(),
            )));
        }
    };
}
