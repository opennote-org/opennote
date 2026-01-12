use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    backup::{base::Backup, scope::BackupScopeIndicator},
    traits::LoadAndSave,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupsStorage {
    pub path: PathBuf,
    pub backups: HashMap<BackupScopeIndicator, Backup>,
}

impl LoadAndSave for BackupsStorage {
    fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            backups: HashMap::new(),
        }
    }

    fn get_path(&self) -> &std::path::Path {
        &self.path
    }
}

impl BackupsStorage {
    /// Return the id of the newly inserted backup
    pub async fn add_backup(&mut self, backup: Backup) -> Result<()> {
        self.backups.insert(backup.scope.clone(), backup);
        self.save().await?;
        Ok(())
    }

    pub fn get_backups_by_scope(&self, scope: &BackupScopeIndicator) -> Vec<Backup> {
        self.backups
            .iter()
            .filter(|(item_scope, _)| {
                if scope.backup_id.is_empty() {
                    item_scope.scope == scope.scope && item_scope.id == scope.id
                } else {
                    *item_scope == scope
                }
            })
            .map(|(_, backup)| backup.clone())
            .collect()
    }

    pub fn get_backup_by_id(&self, id: &str) -> Option<Backup> {
        let item = self.backups.iter().find(|(_, backup)| backup.id == id);

        if let Some((_, backup)) = item {
            return Some(backup.clone());
        }

        None
    }

    pub async fn remove_backups_by_ids(&mut self, ids: &Vec<String>) -> Result<()> {
        self.backups.retain(|_, backup| !ids.contains(&backup.id));

        self.save().await?;
        Ok(())
    }
}
