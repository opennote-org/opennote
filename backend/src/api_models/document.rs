//! It defines the API response and request models of document

use serde::{Deserialize, Serialize};

use crate::documents::document_metadata::DocumentMetadata;

/// region: request

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddDocumentRequest {
    pub username: String,
    pub collection_metadata_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteDocumentRequest {
    pub document_metadata_id: String,
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
    pub document_metadata_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReindexRequest {
    pub username: String,
}

/// region: response

#[derive(Debug, Serialize, Deserialize)]
pub struct ReindexResponse {
    pub documents_reindexed: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddDocumentResponse {
    pub document_metadata_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteDocumentResponse {
    pub document_metadata_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateDocumentResponse {
    pub document_metadata_id: String,
}

/// region: query

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetDocumentsMetadataQuery {
    pub collection_metadata_id: String,
}
