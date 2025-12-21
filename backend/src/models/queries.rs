use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetCollectionsQuery {
    pub username: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetDocumentsMetadataQuery {
    pub collection_metadata_id: String,
}