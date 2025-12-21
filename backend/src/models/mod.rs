pub mod requests;
pub mod responses;
pub mod document_chunk;
pub mod document_metadata;
pub mod collection_metadata;
pub mod queries;
pub mod traits;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub service: String,
    pub version: String,
    pub host: String,
    pub port: u16,
}
