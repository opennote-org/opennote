use actix_web::{HttpResponse, Result, web};
use log::error;
use tokio::sync::RwLock;

use crate::{
    app_state::AppState, callbacks::GenericResponse,
    documents::operations::{intelligent_search_documents, search_documents}, models::requests::SearchDocumentRequest,
    utilities::acquire_data,
};

// Sync endpoint
pub async fn intelligent_search(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<SearchDocumentRequest>,
) -> Result<HttpResponse> {
    // Perform operations synchronously
    // Pull what we need out of AppState without holding the lock during I/O
    let (index_name, db_client, metadata_storage, _, config, user_information_storage) =
        acquire_data(&data).await;

    match intelligent_search_documents(
        &db_client,
        &mut metadata_storage.lock().await,
        &mut user_information_storage.lock().await,
        &index_name,
        &config.embedder,
        &request,
    )
    .await
    {
        Ok(result) => {
            return Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &result)));
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
    let (index_name, db_client, metadata_storage, _, _, user_information_storage) =
        acquire_data(&data).await;

    match search_documents(
        &db_client,
        &mut metadata_storage.lock().await,
        &mut user_information_storage.lock().await,
        &index_name,
        &request,
    )
    .await
    {
        Ok(result) => {
            return Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &result)));
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