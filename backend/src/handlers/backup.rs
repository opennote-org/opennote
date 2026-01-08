use actix_web::{HttpResponse, web};
use anyhow::Result;
use tokio::sync::RwLock;

use crate::{api_models::callbacks::GenericResponse, app_state::AppState, utilities::acquire_data};

// Sync endpoint
pub async fn backup(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, user_information_storage) = acquire_data(&data).await;
    
    // Backup these:
    // 1. User information 
    // 2. All resources under this user
    // 3. Database entries that belongs to this user

    match user_information_storage
        .lock()
        .await
        .update_user_configurations(&request.0.username, request.0.user_configurations)
        .await
    {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &"".to_string())))
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string())))
        }
    }
}

// Sync endpoint
pub async fn restore_backup(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, user_information_storage) = acquire_data(&data).await;

    match user_information_storage
        .lock()
        .await
        .update_user_configurations(&request.0.username, request.0.user_configurations)
        .await
    {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &"".to_string())))
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string())))
        }
    }
}