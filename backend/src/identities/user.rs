use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::configurations::user::UserConfigurations;

/// User system
/// 1. APIs: Login, Logout,
/// 2. Each resource will check against the token retrieved from Login
/// 3. Each resource will unhash the token and check agains the record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub resources: Vec<String>,
    #[serde(default)]
    pub configuration: UserConfigurations,
}

impl User {
    pub fn new(username: String, password: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            password,
            resources: Vec::new(),
            configuration: UserConfigurations::default(),
        }
    }
}

impl From<User> for crate::databases::database::entity::users::ActiveModel {
    fn from(value: User) -> Self {
        Self {
            id: Set(value.id),
            username: Set(value.username),
            password: Set(value.password),
            configuration: Set(serde_json::to_value(&value.configuration).unwrap()),
            resource_ids: Set(serde_json::to_value(&value.resources).unwrap()),
        }
    }
}

impl From<crate::databases::database::entity::users::Model> for User {
    fn from(value: crate::databases::database::entity::users::Model) -> Self {
        Self {
            id: value.id,
            username: value.username,
            password: value.password,
            resources: serde_json::from_value(value.resource_ids).unwrap(),
            configuration: serde_json::from_value(value.configuration).unwrap(),
        }
    }
}
