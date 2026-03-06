use actix_web::{HttpResponse, Result, web};
use log::error;

use crate::{
    api_models::{callbacks::GenericResponse, search::SearchDocumentRequest},
    app_state::AppState,
    documents::operations::retrieve_document_ids_by_scope,
};

// Sync endpoint
pub async fn intelligent_search(
    data: web::Data<AppState>,
    request: web::Json<SearchDocumentRequest>,
) -> Result<HttpResponse> {
    let is_vector_database_valid = match data
        .databases_layer_entry
        .vector_database
        .validate_data_integrity(&data.config.vector_database)
        .await
    {
        Ok(result) => result,
        Err(error) => {
            error!("Failed to validate data integrity: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Failed to validate data integrity. Please try again.".to_string(),
            )));
        }
    };
    
    if !is_vector_database_valid {
        log::warn!("Vector database data is not integral! Trying to recover...");
        match data.databases_layer_entry.recover(&data.config.vector_database).await {
            Ok(_) => (),
            Err(error) => {
                error!("Failed to recover vector database: {}", error);
                return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                    "".to_string(),
                    "Failed to recover vector database. Please try again.".to_string(),
                )));
            }
        }
    }

    let document_metadata_ids: Vec<String> = match retrieve_document_ids_by_scope(
        &data.databases_layer_entry.database,
        request.0.scope.search_scope,
        &request.0.scope.id,
    )
    .await
    {
        Ok(ids) => ids,
        Err(error) => {
            error!("Failed to retrieve document IDs by scope: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Failed to retrieve document IDs. Please try again.".to_string(),
            )));
        }
    };

    if document_metadata_ids.is_empty() {
        log::warn!("No search results found for request {:?}", request);
        let vec: Vec<String> = Vec::new();
        return Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &vec)));
    }

    match data
        .databases_layer_entry
        .vector_database
        .search_documents_semantically(
            &data.databases_layer_entry.database,
            document_metadata_ids,
            &request.0.query,
            request.0.top_n,
            &data.config.embedder.provider,
            &data.config.embedder.base_url,
            &data.config.embedder.api_key,
            &data.config.embedder.model,
            &data.config.embedder.encoding_format,
        )
        .await
    {
        Ok(results) => {
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
    data: web::Data<AppState>,
    request: web::Json<SearchDocumentRequest>,
) -> Result<HttpResponse> {
    let is_vector_database_valid = match data
        .databases_layer_entry
        .vector_database
        .validate_data_integrity(&data.config.vector_database)
        .await
    {
        Ok(result) => result,
        Err(error) => {
            error!("Failed to validate data integrity: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Failed to validate data integrity. Please try again.".to_string(),
            )));
        }
    };
    
    if !is_vector_database_valid {
        match data.databases_layer_entry.recover(&data.config.vector_database).await {
            Ok(_) => (),
            Err(error) => {
                error!("Failed to recover vector database: {}", error);
                return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                    "".to_string(),
                    "Failed to recover vector database. Please try again.".to_string(),
                )));
            }
        }
    }

    let document_metadata_ids: Vec<String> = match retrieve_document_ids_by_scope(
        &data.databases_layer_entry.database,
        request.0.scope.search_scope,
        &request.0.scope.id,
    )
    .await
    {
        Ok(ids) => ids,
        Err(error) => {
            error!("Failed to retrieve document IDs by scope: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Failed to retrieve document IDs. Please try again.".to_string(),
            )));
        }
    };

    if document_metadata_ids.is_empty() {
        log::warn!("No search results found for request {:?}", request);
        let vec: Vec<String> = Vec::new();
        return Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &vec)));
    }

    match data
        .databases_layer_entry
        .vector_database
        .search_documents(
            &data.databases_layer_entry.database,
            &document_metadata_ids,
            &request.0.query,
            request.0.top_n,
        )
        .await
    {
        Ok(results) => {
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
