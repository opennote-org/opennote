use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::configurations::user::UserConfigurations;

/// User system
/// 1. APIs: Login, Logout,
/// 2. Each resource will check against the token retrieved from Login
/// 3. Each resource will unhash the token and check agains the record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
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
