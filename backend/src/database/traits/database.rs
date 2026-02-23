use anyhow::Result;
use async_trait::async_trait;

use crate::{
    database::traits::{identities::Identities, metadata::MetadataManagement},
    identities::storage::IdentitiesStorage,
    metadata_storage::MetadataStorage,
    vector_database::traits::VectorDatabase,
};

#[async_trait]
pub trait Database: MetadataManagement + Identities + Send + Sync {
    async fn migrate(
        &self,
        metadata_storage_path: &str,
        identities_storage_path: &str,
        vector_database: &dyn VectorDatabase,
    ) -> Result<()>;

    async fn migrate_users(&self, identities_storage: &IdentitiesStorage) -> Result<()>;

    async fn migrate_collections(&self, metadata_storage: &MetadataStorage) -> Result<()>;

    async fn migrate_documents(
        &self,
        metadata_storage: &MetadataStorage,
        vector_database: &dyn VectorDatabase,
    ) -> Result<()>;

    async fn migrate_metadata_settings(&self, metadata_storage: &MetadataStorage) -> Result<()>;
}
