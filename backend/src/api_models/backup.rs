//! It defines request and response API models for backup

use serde::{Deserialize, Serialize};

use crate::backup::scope::BackupScopeIndicator;

/// region: request

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupRequest {
    pub scope: BackupScopeIndicator,
}

/// region: response

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupResponse {}