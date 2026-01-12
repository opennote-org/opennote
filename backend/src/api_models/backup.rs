//! It defines request and response API models for backup

use serde::{Deserialize, Serialize};

use crate::backup::{list_item::BackupListItem, scope::BackupScopeIndicator};

/// region: request

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupRequest {
    pub scope: BackupScopeIndicator,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RestoreBackupRequest {
    pub backup_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetBackupsListRequest {
    pub scope: BackupScopeIndicator,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoveBackupsRequest {
    pub backup_ids: Vec<String>,
}

/// region: response

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupResponse {
    pub backup_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetBackupsListResponse {
    pub backups: Vec<BackupListItem>,
}
