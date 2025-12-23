use std::path::PathBuf;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{
    configurations::user::UserConfigurations, identities::user::User, traits::LoadAndSave,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInformationStorage {
    pub path: PathBuf,
    pub users: Vec<User>,
}

impl LoadAndSave for UserInformationStorage {
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

/// The allowed dead codes are reserved for future upgrades
impl UserInformationStorage {
    /// Username must be unique
    pub async fn create_user(&mut self, username: String, password: String) -> Result<()> {
        if self.users.iter().any(|user| *user.username == username) {
            return Err(anyhow!("Username `{}` has already existed", username));
        }

        self.users.push(User::new(username, password));
        self.save().await?;
        Ok(())
    }

    pub fn validate_user_password(&self, username: &str, password: &str) -> Result<bool> {
        if let Some(user) = self.users.iter().find(|user| user.username == username) {
            return Ok(user.password == password);
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    pub async fn add_authorized_resources(
        &mut self,
        username: &str,
        resource_ids: Vec<String>,
    ) -> Result<()> {
        if let Some(user) = self.users.iter_mut().find(|user| user.username == username) {
            user.resources.extend(resource_ids);
            self.save().await?;
            return Ok(());
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    pub async fn remove_authorized_resources(
        &mut self,
        username: &str,
        resource_ids: Vec<String>,
    ) -> Result<()> {
        if let Some(user) = self.users.iter_mut().find(|user| user.username == username) {
            user.resources
                .retain(|resource_id| !resource_ids.contains(resource_id));
            self.save().await?;
            return Ok(());
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    /// See if the user has the permission to access the given resources
    pub fn check_permission(&self, username: &str, resource_ids: Vec<String>) -> Result<bool> {
        if let Some(user) = self.users.iter().find(|user| user.username == username) {
            if resource_ids
                .iter()
                .all(|resource_id| user.resources.contains(resource_id))
            {
                return Ok(true);
            }

            return Ok(false);
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    pub async fn update_user_configurations(
        &mut self,
        username: &str,
        user_configurations: UserConfigurations,
    ) -> Result<()> {
        if let Some(user) = self.users.iter_mut().find(|user| user.username == username) {
            user.configuration = user_configurations;
            self.save().await?;
            return Ok(());
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    pub async fn get_user_configurations(&self, username: &str) -> Result<UserConfigurations> {
        if let Some(user) = self.users.iter().find(|user| user.username == username) {
            return Ok(user.configuration.clone());
        }

        Err(anyhow!("User `{}` does not exist", username))
    }

    pub fn get_users_by_resource_id(&self, id: &str) -> Vec<&User> {
        self.users
            .iter()
            .filter(|user| user.resources.contains(&id.to_string()))
            .collect()
    }

    pub fn get_resource_ids_by_username(&self, username: &str) -> Vec<&String> {
        let mut resource_ids = Vec::new();

        for user in self.users.iter() {
            if user.username == username {
                resource_ids = user.resources.iter().collect();
            }
        }

        resource_ids
    }
}
