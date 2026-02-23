use anyhow::Result;
use async_trait::async_trait;

use crate::{
    database::{
        filters::{get_collections::GetCollectionFilter, get_documents::GetDocumentFilter},
        metadata::MetadataSettings,
    },
    documents::{collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata},
};

/// it defines methods for managing metadata
#[async_trait]
pub trait MetadataManagement {
    async fn create_collection(&self, title: &str) -> Result<String>;

    async fn delete_collections(
        &self,
        collection_metadata_ids: &Vec<String>,
    ) -> Result<Vec<CollectionMetadata>>;

    async fn delete_documents(
        &self,
        document_metadata_ids: &Vec<String>,
    ) -> Result<Vec<DocumentMetadata>>;

    async fn update_collections(
        &self,
        mut collection_metadatas: Vec<CollectionMetadata>,
    ) -> Result<()>;

    /// For updating chunks in document metadatas.
    /// The update method won't allow mutate the chunks for security reason.
    async fn update_documents_with_new_chunks(
        &self,
        document_metadatas: Vec<DocumentMetadata>,
    ) -> Result<()>;

    /// Prevent immutable fields to get accidentally mutated
    async fn verify_immutable_fields_in_document_metadatas(
        &self,
        document_metadatas: &mut Vec<DocumentMetadata>,
    ) -> Result<()>;

    async fn update_documents(&self, document_metadatas: Vec<DocumentMetadata>) -> Result<()>;

    async fn update_metadata_settings(
        &self,
        metadata_settings: MetadataSettings,
    ) -> Result<MetadataSettings>;

    async fn add_collections(&self, collection_metadatas: Vec<CollectionMetadata>) -> Result<()>;

    async fn add_documents(&self, document_metadatas: Vec<DocumentMetadata>) -> Result<()>;

    async fn get_documents(&self, filter: GetDocumentFilter) -> Result<Vec<DocumentMetadata>>;

    async fn get_collections(
        &self,
        filter: GetCollectionFilter,
        include_chunk_data: bool,
    ) -> Result<Vec<CollectionMetadata>>;

    async fn get_metadata_settings(&self) -> Result<MetadataSettings>;
}
