use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    backup::storage::BackupsStorage, configurations::system::Config, database::Database,
    identities::storage::IdentitiesStorage, metadata_storage::MetadataStorage,
    tasks_scheduler::TasksScheduler, traits::LoadAndSave,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
    pub database: Database,
    pub backups_storage: Arc<Mutex<BackupsStorage>>,
    pub metadata_storage: Arc<Mutex<MetadataStorage>>,
    pub identities_storage: Arc<Mutex<IdentitiesStorage>>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config_clone = config.clone();

        Ok(Self {
            config,
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            database: Database::new(&config_clone).await?,
            backups_storage: Arc::new(Mutex::new(BackupsStorage::load(
                &config_clone.backups_storage.path,
            )?)),
            metadata_storage: Arc::new(Mutex::new(MetadataStorage::load(
                &config_clone.metadata_storage.path,
            )?)),
            identities_storage: Arc::new(Mutex::new(IdentitiesStorage::load(
                &config_clone.identities_storage.path,
            )?)),
        })
    }
}
