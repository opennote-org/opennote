use serde::{Deserialize, Serialize};

use crate::backup::{base::Backup, scope::BackupScopeIndicator};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupListItem {
    pub id: String,
    pub created_at: String,
    pub scope: BackupScopeIndicator,
}

impl From<Backup> for BackupListItem {
    fn from(value: Backup) -> Self {
        Self {
            id: value.id,
            created_at: value.created_at,
            scope: value.scope,
        }
    }
}
