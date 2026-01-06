use serde::{Deserialize, Serialize};

use crate::configurations::user::UserConfigurations;

/// region: request

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateUserConfigurationsRequest {
    pub username: String,
    pub user_configurations: UserConfigurations,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetUserConfigurationsRequest {
    pub username: String,
}

/// region: response

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginResponse {
    pub is_login: bool,
}
