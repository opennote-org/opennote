//! It defines the API requests and response models of collection

use serde::{Deserialize, Serialize};

use crate::documents::collection_metadata::CollectionMetadata;

/// region: requests

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateCollectionMetadataRequest {
    pub collection_metadatas: Vec<CollectionMetadata>,
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

/// region: responses

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateCollectionResponse {
    pub collection_metadata_id: String,
}

/// region: query

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetCollectionsQuery {
    pub username: String,
}
