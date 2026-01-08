use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    backup::scope::{BackupScope, BackupScopeIndicator},
    documents::{collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata},
    identities::user::User,
};

/// Represents a copy of a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archieve {
    pub scope: BackupScope,
    pub user_information_snapshots: Vec<User>,
    pub collection_metadata_snapshots: HashMap<String, CollectionMetadata>,
    pub document_metadata_snapshots: HashMap<String, DocumentMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchievesStorage {
    pub archieves: HashMap<BackupScopeIndicator, Archieve>,
}
