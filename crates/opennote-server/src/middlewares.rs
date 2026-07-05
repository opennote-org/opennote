use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::AUTHORIZATION,
    middleware::Next,
};

use opennote_models::constants::env_vars::{
    SERVER_PASSWORD_ENV_VAR_NAME, load_environment_variable,
};

/// Check the password when a connection goes in
pub async fn check_password(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // pre-processing
    let auth_value = match req.headers().get(AUTHORIZATION) {
        Some(val) => val,
        None => {
            return Err(actix_web::error::ErrorUnauthorized(
                "Missing authorization header",
            ));
        }
    };

    let password = load_environment_variable(SERVER_PASSWORD_ENV_VAR_NAME);

    if auth_value.to_str().unwrap_or("") != password {
        return Err(actix_web::error::ErrorUnauthorized("Invalid password"));
    }

    next.call(req).await
    // post-processing
}
