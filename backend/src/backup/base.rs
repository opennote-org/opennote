use std::collections::HashMap;

use actix_web::cookie::time::UtcDateTime;
use serde::{Deserialize, Serialize};

use crate::{
    backup::scope::BackupScopeIndicator,
    documents::{
        collection_metadata::CollectionMetadata, document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    },
    identities::user::User,
};

/// Represents a copy of a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub id: String,
    pub created_at: String,
    pub scope: BackupScopeIndicator,
    pub user_information_snapshots: Vec<User>,
    pub collection_metadata_snapshots: HashMap<String, CollectionMetadata>,
    pub document_metadata_snapshots: HashMap<String, DocumentMetadata>,
    pub document_chunks_snapshots: Vec<DocumentChunk>,
}

impl Backup {
    pub fn new(
        scope: BackupScopeIndicator,
        user_information_snapshots: Vec<User>,
        collection_metadata_snapshots: HashMap<String, CollectionMetadata>,
        document_metadata_snapshots: HashMap<String, DocumentMetadata>,
        document_chunks_snapshots: Vec<DocumentChunk>,
    ) -> Self {
        Self {
            id: scope.backup_id.clone(),
            created_at: UtcDateTime::now().to_string(),
            scope,
            user_information_snapshots,
            collection_metadata_snapshots,
            document_metadata_snapshots,
            document_chunks_snapshots,
        }
    }

    pub fn get_usernames(&self) -> Vec<String> {
        self.user_information_snapshots
            .iter()
            .map(|item| item.username.clone())
            .collect()
    }

    pub fn get_collection_metadata_ids(&self) -> Vec<String> {
        self.collection_metadata_snapshots
            .iter()
            .map(|(collection_metadata_id, _)| collection_metadata_id.to_owned())
            .collect()
    }

    pub fn get_document_metadata_ids(&self) -> Vec<String> {
        self.document_metadata_snapshots
            .iter()
            .map(|(document_metadata_id, _)| document_metadata_id.to_owned())
            .collect()
    }
}
