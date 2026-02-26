use actix_web::{
    HttpResponse, Result,
    web::{self, Query},
};
use serde_json::json;

use crate::{
    api_models::{
        callbacks::GenericResponse,
        collection::{
            CreateCollectionRequest, CreateCollectionResponse, DeleteCollectionRequest,
            GetCollectionsQuery, UpdateCollectionMetadataRequest,
        },
    },
    app_state::AppState,
    documents::collection_metadata::CollectionMetadata,
};

// Sync Endpoint
pub async fn create_collection(
    data: web::Data<AppState>,
    request: web::Json<CreateCollectionRequest>,
) -> Result<HttpResponse> {
    match data
        .database
        .create_collection(&request.collection_title)
        .await
    {
        Ok(collection_metadata_id) => {
            match data
                .database
                .add_authorized_resources(&request.username, vec![collection_metadata_id.clone()])
                .await
            {
                Ok(_) => {}
                Err(error) => {
                    log::error!("Failed to add authorized resources: {}", error);
                    return Ok(HttpResponse::Ok()
                        .json(GenericResponse::fail("".to_string(), error.to_string())));
                }
            }

            return Ok(HttpResponse::Ok().json(GenericResponse::succeed(
                "".to_string(),
                &CreateCollectionResponse {
                    collection_metadata_id,
                },
            )));
        }
        Err(error) => {
            return Ok(
                HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string()))
            );
        }
    };
}

/// Sync endpoint
pub async fn delete_collections(
    data: web::Data<AppState>,
    request: web::Json<DeleteCollectionRequest>,
) -> Result<HttpResponse> {
    let collection_metadata = match data
        .database
        .delete_collections(&request.collection_metadata_ids)
        .await
    {
        Ok(collection_metadata) => {
            if collection_metadata.is_empty() {
                return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                    "".to_string(),
                    "Collection metadata id was not found. Please specify an existing collection"
                        .to_string(),
                )));
            }

            match data
                .database
                .remove_authorized_resources(
                    &request.0.username,
                    &request.0.collection_metadata_ids,
                )
                .await
            {
                Ok(_) => {}
                Err(error) => log::warn!(
                    "Username not found when trying to remove resources {:?} from it: {}",
                    request.0.collection_metadata_ids,
                    error
                ),
            }

            match data
                .vector_database
                .delete_documents_from_database(
                    &data.config.vector_database,
                    &collection_metadata
                        .iter()
                        .flat_map(|item| item.documents_metadatas.iter().map(|item| item.id))
                        .collect(),
                )
                .await
            {
                Ok(_) => {}
                Err(error) => log::error!("Vector database cannot delete documents due to {}", error),
            }

            collection_metadata
        }
        Err(error) => {
            log::error!("Error happened when deleting the collection: {}", error);
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Error happened when deleting the collection".to_string(),
            )));
        }
    };

    return Ok(HttpResponse::Ok().json(GenericResponse::succeed(
        "".to_string(),
        &collection_metadata,
    )));
}

/// Sync endpoint
pub async fn get_collections(
    data: web::Data<AppState>,
    query: Query<GetCollectionsQuery>,
) -> Result<HttpResponse> {
    let collections = match data
        .database
        .get_collections_by_collection_metadata_id()
        .await
    {
        Ok(collections) => collections,
        Err(error) => {
            log::error!("Failed to get collections: {}", error);
            return Ok(
                HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string()))
            );
        }
    };

    let collection_metadata: Vec<CollectionMetadata> = collections
        .iter()
        .filter(async |collection| {
            data.database
                .check_permission(&query.username, vec![collection.id.clone()])
                .await
                .unwrap()
        })
        .map(|collection| collection.to_owned())
        .collect();

    // Return an immediate response with a task id
    Ok(HttpResponse::Ok()
        .json(GenericResponse::succeed(
            "".to_string(),
            &collection_metadata,
        ))
        .into())
}

/// Sync endpoint
pub async fn update_collections_metadata(
    data: web::Data<AppState>,
    request: web::Json<UpdateCollectionMetadataRequest>,
) -> Result<HttpResponse> {
    let (_, metadata_storage, _, _, _, _) = acquire_data(&data).await;

    match metadata_storage
        .lock()
        .await
        .update_collection(request.0.collection_metadatas)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &json!({})))),
        Err(error) => {
            log::error!("Failed when trying updating documents metadata: {}", error);
            return Ok(
                HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string()))
            );
        }
    }
}
