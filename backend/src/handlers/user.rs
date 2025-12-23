use actix_web::{HttpResponse, Result, web};
use tokio::sync::RwLock;

use crate::{
    api_models::{
        callbacks::GenericResponse,
        user::{
            CreateUserRequest, GetUserConfigurationsRequest, LoginRequest, LoginResponse,
            UpdateUserConfigurationsRequest,
        },
    },
    app_state::AppState,
    utilities::acquire_data,
};

// Sync endpoint
pub async fn create_user(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<CreateUserRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, user_information_storage) = acquire_data(&data).await;

    match user_information_storage
        .lock()
        .await
        .create_user(request.username.clone(), request.password.clone())
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
pub async fn login(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<LoginRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, user_information_storage) = acquire_data(&data).await;

    match user_information_storage
        .lock()
        .await
        .validate_user_password(&request.username, &request.password)
    {
        Ok(result) => {
            let login_response: LoginResponse = LoginResponse { is_login: result };
            Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &login_response)))
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string())))
        }
    }
}

// Sync endpoint
pub async fn get_user_configurations(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<GetUserConfigurationsRequest>,
) -> Result<HttpResponse> {
    // Pull what we need out of AppState without holding the lock during I/O
    let (_, _, _, _, _, user_information_storage) = acquire_data(&data).await;

    match user_information_storage
        .lock()
        .await
        .get_user_configurations(&request.0.username)
        .await
    {
        Ok(result) => {
            Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &result)))
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string())))
        }
    }
}

// Sync endpoint
pub async fn update_user_configurations(
    data: web::Data<RwLock<AppState>>,
    request: web::Json<UpdateUserConfigurationsRequest>,
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
