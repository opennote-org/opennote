use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::{
    databases::{
        database::traits::{identities::Identities, metadata::MetadataManagement},
        vector_database::traits::VectorDatabase,
    },
    identities::storage::IdentitiesStorage,
    metadata_storage::MetadataStorage,
};

#[async_trait]
pub trait Database: MetadataManagement + Identities + Send + Sync {
    async fn migrate(
        &self,
        metadata_storage_path: &str,
        identities_storage_path: &str,
        vector_database: &Arc<dyn VectorDatabase>,
    ) -> Result<()>;

    async fn migrate_users(&self, identities_storage: &IdentitiesStorage) -> Result<()>;

    async fn migrate_collections(&self, metadata_storage: &MetadataStorage) -> Result<()>;

    async fn migrate_documents(
        &self,
        metadata_storage: &MetadataStorage,
        vector_database: &Arc<dyn VectorDatabase>,
    ) -> Result<()>;

    async fn migrate_metadata_settings(&self, metadata_storage: &MetadataStorage) -> Result<()>;

    async fn create_tables(&self) -> Result<()>;
}
