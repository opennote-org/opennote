use actix_web::{HttpResponse, Result, web};
use schemars::schema_for;

use crate::{
    api_models::{
        callbacks::GenericResponse,
        user::{
            CreateUserRequest, GetUserConfigurationsRequest, LoginRequest, LoginResponse,
            UpdateUserConfigurationsRequest,
        },
    },
    app_state::AppState,
    configurations::user::UserConfigurations,
    databases::database::filters::get_users::GetUserFilter,
};

// Sync endpoint
pub async fn create_user(
    data: web::Data<AppState>,
    request: web::Json<CreateUserRequest>,
) -> Result<HttpResponse> {
    match data
        .databases_layer_entry
        .database
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
    data: web::Data<AppState>,
    request: web::Json<LoginRequest>,
) -> Result<HttpResponse> {
    match data
        .databases_layer_entry
        .database
        .validate_user_password(&request.username, &request.password)
        .await
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
    data: web::Data<AppState>,
    request: web::Json<GetUserConfigurationsRequest>,
) -> Result<HttpResponse> {
    match data
        .databases_layer_entry
        .database
        .get_users(&GetUserFilter {
            usernames: vec![request.0.username],
            ..Default::default()
        })
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(GenericResponse::succeed(
            "".to_string(),
            &result[0].configuration,
        ))),
        Err(error) => {
            Ok(HttpResponse::Ok().json(GenericResponse::fail("".to_string(), error.to_string())))
        }
    }
}

// Sync endpoint
pub async fn update_user_configurations(
    data: web::Data<AppState>,
    request: web::Json<UpdateUserConfigurationsRequest>,
) -> Result<HttpResponse> {
    match data
        .databases_layer_entry
        .database
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
pub async fn get_user_configurations_schemars() -> Result<HttpResponse> {
    let schemars: schemars::Schema = schema_for!(UserConfigurations);
    Ok(HttpResponse::Ok().json(GenericResponse::succeed("".to_string(), &schemars)))
}
