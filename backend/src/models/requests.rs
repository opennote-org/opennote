use serde::{Deserialize, Serialize};

use crate::configurations::user::UserConfigurations;

use super::{collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddDocumentRequest {
    pub username: String,
    pub collection_metadata_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteDocumentRequest {
    pub document_metadata_id: String
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateCollectionMetadataRequest {
    pub collection_metadatas: Vec<CollectionMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateDocumentMetadataRequest {
    pub document_metadatas: Vec<DocumentMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateDocumentContentRequest {
    pub username: String,
    pub document_metadata_id: String,
    pub collection_metadata_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetDocumentRequest {
    pub document_metadata_id: String
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    Document,
    Collection,
    Userspace,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchScopeIndicator {
    pub search_scope: SearchScope,
    pub id: String, 
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchDocumentRequest {
    pub query: String,
    pub top_n: usize,
    pub scope: SearchScopeIndicator,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateCollectionRequest {
    pub username: String,
    pub collection_title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteCollectionRequest {
    pub collection_metadata_id: String,
}

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
    pub user_configurations: UserConfigurations
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetUserConfigurationsRequest {
    pub username: String,
}