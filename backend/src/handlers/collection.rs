use actix_web::{
    HttpResponse, Result,
    web::{self, Query},
};
use qdrant_client::qdrant::{Condition, DeletePointsBuilder, Filter};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{
    api_models::{callbacks::GenericResponse, collection::{
        CreateCollectionRequest, CreateCollectionResponse, DeleteCollectionRequest,
        GetCollectionsQuery, UpdateCollectionMetadataRequest,
    }},
    app_state::AppState,
    documents::collection_metadata::CollectionMetadata,
    utilities::acquire_data,
};

// Sync Endpoint
pub async fn create_collection(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<CreateCollectionRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, metadata_storage, _, _, user_information_storage) = acquire_data(&data).await;

    match metadata_storage
        .lock()
        .await
        .create_collection(&request.collection_title)
        .await
    {
        Ok(collection_metadata_id) => {
            match user_information_storage
                .lock()
                .await
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
///
/// TODO: need to delete all belonging documents under the collection from the database
pub async fn delete_collection(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<DeleteCollectionRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (index, client, metadata_storage, _, _, user_information_storage) =
        acquire_data(&data).await;

    let collection_metadata = match metadata_storage
        .lock()
        .await
        .delete_collection(&request.collection_metadata_id)
        .await
    {
        Some(collection_metadata) => {
            let usernames: Vec<String> = user_information_storage
                .lock()
                .await
                .get_users_by_resource_id(&request.0.collection_metadata_id)
                .iter()
                .map(|user| user.username.clone())
                .collect();

            let mut user_information_storage = user_information_storage.lock().await;
            for username in usernames {
                match user_information_storage
                    .remove_authorized_resources(
                        &username,
                        vec![request.0.collection_metadata_id.clone()],
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(error) => log::warn!(
                        "Username not found when trying to remove resource {} from it: {}",
                        request.0.collection_metadata_id,
                        error
                    ),
                }
            }

            collection_metadata
        }
        None => {
            return Ok(HttpResponse::Ok().json(GenericResponse::fail(
                "".to_string(),
                "Collection metadata id was not found. Please specify an existing collection"
                    .to_string(),
            )));
        }
    };

    let mut conditions: Vec<Condition> = Vec::new();
    for id in collection_metadata.documents_metadata_ids.iter() {
        conditions.push(Condition::matches("document_metadata_id", id.to_string()));
    }

    match client
        .delete_points(
            DeletePointsBuilder::new(&index)
                .points(Filter::any(conditions))
                .wait(true),
        )
        .await
    {
        Ok(_) => {}
        Err(error) => log::error!("Qdrant cannot delete documents due to {}", error),
    }

    return Ok(HttpResponse::Ok().json(GenericResponse::succeed(
        "".to_string(),
        &collection_metadata,
    )));
}

/// Sync endpoint
pub async fn get_collections(
    data: web::Data<RwLock<AppState>>,
    query: Query<GetCollectionsQuery>,
) -> Result<HttpResponse> {
    let (_, _, metadata_storage, _, _, user_information_storage) = acquire_data(&data).await;

    let guard = user_information_storage.lock().await;

    let collection_metadata: Vec<CollectionMetadata> = metadata_storage
        .lock()
        .await
        .collections
        .iter()
        .filter(|(_, collection)| {
            guard
                .check_permission(&query.username, vec![collection.metadata_id.clone()])
                .unwrap()
        })
        .map(|(_, collection)| collection.to_owned())
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
    data: web::Data<RwLock<AppState>>,
    request: web::Json<UpdateCollectionMetadataRequest>,
) -> Result<HttpResponse> {
    let (_, _, metadata_storage, _, _, _) = acquire_data(&data).await;

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
