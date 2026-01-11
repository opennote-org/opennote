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
