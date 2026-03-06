use anyhow::Result;
use async_trait::async_trait;

use crate::{
    configurations::user::UserConfigurations, databases::database::filters::get_users::GetUserFilter,
    identities::user::User,
};

#[async_trait]
pub trait Identities {
    /// Username must be unique
    async fn create_user(&self, username: String, password: String) -> Result<()>;

    async fn validate_user_password(&self, username: &str, password: &str) -> Result<bool>;

    async fn add_authorized_resources(
        &self,
        username: &str,
        resource_ids: Vec<String>,
    ) -> Result<()>;

    async fn remove_authorized_resources(
        &self,
        username: &str,
        resource_ids: &Vec<String>,
    ) -> Result<()>;

    async fn update_user_configurations(
        &self,
        username: &str,
        user_configurations: UserConfigurations,
    ) -> Result<()>;

    async fn get_resource_ids_by_username(&self, username: &str) -> Result<Vec<String>>;

    /// Return false if the username does not exist or not owning the specified collections.
    /// Vice versa.
    async fn is_user_owning_collections(
        &self,
        username: &str,
        collection_metadata_ids: &[String],
    ) -> Result<bool>;

    /// This method will delete users, as well as its owning resources/collections
    async fn delete_users(&self, usernames: Vec<String>) -> Result<Vec<User>>;

    async fn add_users(&self, users: Vec<User>) -> Result<()>;

    async fn get_users(&self, filter: &GetUserFilter) -> Result<Vec<User>>;
}
