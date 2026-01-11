use std::{collections::HashMap, path::PathBuf};

use actix_web::cookie::time::UtcDateTime;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    backup::scope::BackupScopeIndicator,
    documents::{
        collection_metadata::CollectionMetadata, document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    },
    identities::user::User,
    traits::LoadAndSave,
};

/// Represents a copy of a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archieve {
    pub id: String,
    pub created_at: String,
    pub scope: BackupScopeIndicator,
    pub user_information_snapshots: Vec<User>,
    pub collection_metadata_snapshots: HashMap<String, CollectionMetadata>,
    pub document_metadata_snapshots: HashMap<String, DocumentMetadata>,
    pub document_chunks_snapshots: Vec<DocumentChunk>,
}

impl Archieve {
    pub fn new(
        scope: BackupScopeIndicator,
        user_information_snapshots: Vec<User>,
        collection_metadata_snapshots: HashMap<String, CollectionMetadata>,
        document_metadata_snapshots: HashMap<String, DocumentMetadata>,
        document_chunks_snapshots: Vec<DocumentChunk>,
    ) -> Self {
        Self {
            id: scope.archieve_id.clone(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchievesStorage {
    pub path: PathBuf,
    pub archieves: HashMap<BackupScopeIndicator, Archieve>,
}

impl LoadAndSave for ArchievesStorage {
    fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            archieves: HashMap::new(),
        }
    }

    fn get_path(&self) -> &std::path::Path {
        &self.path
    }
}

impl ArchievesStorage {
    /// Return the id of the newly inserted archieve
    pub async fn add_archieve(&mut self, archieve: Archieve) -> Result<()> {
        self.archieves.insert(archieve.scope.clone(), archieve);
        self.save().await?;
        Ok(())
    }

    pub fn get_archieves_by_scope(&self, scope: &BackupScopeIndicator) -> Vec<Archieve> {
        self.archieves
            .iter()
            .filter(|(item_scope, _)| {
                if scope.archieve_id.is_empty() {
                    item_scope.scope == scope.scope && item_scope.id == scope.id
                } else {
                    *item_scope == scope
                }
            })
            .map(|(_, archieve)| archieve.clone())
            .collect()
    }

    pub fn get_archieve_by_id(&self, id: &str) -> Option<Archieve> {
        let item = self
            .archieves
            .iter()
            .find(|(_, archieve)| archieve.id == id);

        if let Some((_, archieve)) = item {
            return Some(archieve.clone());
        }

        None
    }

    pub async fn remove_archieves_by_ids(&mut self, ids: &Vec<String>) -> Result<()> {
        self.archieves
            .retain(|_, archieve| !ids.contains(&archieve.id));

        self.save().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArchieveListItem {
    pub id: String,
    pub created_at: String,
    pub scope: BackupScopeIndicator,
}

impl From<Archieve> for ArchieveListItem {
    fn from(value: Archieve) -> Self {
        Self {
            id: value.id,
            created_at: value.created_at,
            scope: value.scope,
        }
    }
}
