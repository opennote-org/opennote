use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BackupScope {
    User,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct BackupScopeIndicator {
    pub search_scope: BackupScope,
    pub id: String,
}