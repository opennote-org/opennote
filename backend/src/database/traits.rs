use anyhow::Result;
use async_trait::async_trait;

use crate::{database::metadata::MetadataSettings, documents::{
    collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata,
}};

/// it defines methods for managing metadata
#[async_trait]
pub trait MetadataManagement {
    async fn create_collection(&mut self, title: &str) -> Result<String>;

    /// Return None if the given metadata id is not found
    async fn delete_collection(
        &mut self,
        collection_metadata_id: &str,
    ) -> Option<CollectionMetadata>;

    async fn update_collection(
        &mut self,
        mut collection_metadatas: Vec<CollectionMetadata>,
    ) -> Result<()>;

    /// For updating chunks in document metadatas.
    /// The update method won't allow mutate the chunks for security reason.
    async fn update_documents_with_new_chunks(
        &mut self,
        document_metadatas: Vec<DocumentMetadata>,
    ) -> Result<()>;

    /// Prevent immutable fields to get accidentally mutated
    async fn verify_immutable_fields_in_document_metadatas(
        &self,
        document_metadatas: &mut Vec<DocumentMetadata>,
    ) -> Result<()>;

    async fn update_documents(&mut self, document_metadatas: Vec<DocumentMetadata>) -> Result<()>;

    async fn add_document(&mut self, metadata: DocumentMetadata) -> Result<()>;

    async fn get_document(&self, docuemnt_metadata_id: &str) -> Option<DocumentMetadata>;

    async fn remove_document(&mut self, metdata_id: &str) -> Option<DocumentMetadata>;

    async fn get_document_ids_by_collection(&self, collection_metadata_id: &str) -> Vec<String>;
    
    async fn get_number_documents(&self) -> Result<usize>;
    
    async fn get_metadata_settings(&self) -> Result<MetadataSettings>;
    
    async fn update_metadata_settings(&self, metadata_settings: MetadataSettings) -> Result<MetadataSettings>;
}
