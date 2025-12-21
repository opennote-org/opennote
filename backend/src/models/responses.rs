use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateCollectionResponse {
    pub collection_metadata_id: String, 
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginResponse {
    pub is_login: bool, 
}
