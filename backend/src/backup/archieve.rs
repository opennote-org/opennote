use std::{collections::HashMap, path::PathBuf};

use actix_web::cookie::time::UtcDateTime;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
            id: Uuid::new_v4().to_string(),
            created_at: UtcDateTime::now().to_string(),
            scope,
            user_information_snapshots,
            collection_metadata_snapshots,
            document_metadata_snapshots,
            document_chunks_snapshots,
        }
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
    pub async fn add_archieve(&mut self, archieve: Archieve) -> Result<()> {
        self.archieves.insert(archieve.scope.clone(), archieve);
        self.save().await?;
        Ok(())
    }

    pub fn get_archieves_by_scope(&self, scope: &BackupScopeIndicator) -> Vec<Archieve> {
        self.archieves
            .iter()
            .filter(|(item_scope, _)| *item_scope == scope)
            .map(|(_, archieve)| archieve.clone())
            .collect()
    }
    
    pub fn get_archieve_by_id(&self, id: &str) -> Option<Archieve> {
        let item = self.archieves.iter()
            .find(|(_, archieve)| archieve.id == id);
        
        if let Some((_, archieve)) = item {
            return Some(archieve.clone());
        }
        
        None
    }
    
    pub async fn remove_archieve_by_id(&mut self, id: &str) -> Result<()> {
        let scope = self.archieves.iter()
            .find(|(_, archieve)| archieve.id == id)
            .map(|(scope, _)| scope.to_owned());
        
        if let Some(scope) = scope {
            self.archieves.remove(&scope);
        }
        
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
