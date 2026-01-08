//! It defines request and response API models for backup

use serde::{Deserialize, Serialize};

use crate::backup::{archieve::ArchieveListItem, scope::BackupScopeIndicator};

/// region: request

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupRequest {
    pub scope: BackupScopeIndicator,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetBackupsListRequest {
    pub scope: BackupScopeIndicator,
}

/// region: response

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupResponse {
    pub archieve_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetBackupsListResponse {
    pub archieves: Vec<ArchieveListItem>,
}