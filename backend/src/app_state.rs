use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    backup::archieve::ArchievesStorage, configurations::system::Config, database::Database,
    identities::storage::UserInformationStorage, metadata_storage::MetadataStorage,
    tasks_scheduler::TasksScheduler, traits::LoadAndSave,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
    pub database: Database,
    pub archieve_storage: Arc<Mutex<ArchievesStorage>>,
    pub metadata_storage: Arc<Mutex<MetadataStorage>>,
    pub user_information_storage: Arc<Mutex<UserInformationStorage>>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config_clone = config.clone();

        Ok(Self {
            config,
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            database: Database::new(&config_clone).await?,
            archieve_storage: Arc::new(Mutex::new(ArchievesStorage::load(
                &config_clone.archieve_storage.path,
            )?)),
            metadata_storage: Arc::new(Mutex::new(MetadataStorage::load(
                &config_clone.metadata_storage.path,
            )?)),
            user_information_storage: Arc::new(Mutex::new(UserInformationStorage::load(
                &config_clone.user_information_storage.path,
            )?)),
        })
    }
}
