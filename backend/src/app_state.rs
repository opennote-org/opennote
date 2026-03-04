use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    backup::storage::BackupsStorage,
    configurations::system::Config,
    databases::entry::DatabasesLayerEntry,
    tasks_scheduler::TasksScheduler,
    traits::LoadAndSave,
    vector_database::{shared::create_vector_database, traits::VectorDatabase},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub databases_layer_entry: DatabasesLayerEntry,
    pub backups_storage: Arc<Mutex<BackupsStorage>>,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            backups_storage: Arc::new(Mutex::new(BackupsStorage::load(
                &config.backups_storage.path,
            )?)),
            databases_layer_entry: DatabasesLayerEntry::new(&config),
        })
    }
}
