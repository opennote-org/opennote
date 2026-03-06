use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{identities::user::User, traits::LoadAndSave};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitiesStorage {
    pub path: PathBuf,
    pub users: Vec<User>,
}

impl LoadAndSave for IdentitiesStorage {
    fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            users: Vec::new(),
        }
    }

    fn get_path(&self) -> &std::path::Path {
        &self.path
    }
}
