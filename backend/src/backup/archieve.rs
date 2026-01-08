use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    backup::scope::BackupScopeIndicator,
    documents::{collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata},
    identities::user::User,
    traits::LoadAndSave,
};

/// Represents a copy of a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archieve {
    pub scope: BackupScopeIndicator,
    pub user_information_snapshots: Vec<User>,
    pub collection_metadata_snapshots: HashMap<String, CollectionMetadata>,
    pub document_metadata_snapshots: HashMap<String, DocumentMetadata>,
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
}