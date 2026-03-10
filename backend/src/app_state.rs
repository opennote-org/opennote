use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    configurations::system::Config, databases::entry::DatabasesLayerEntry,
    tasks_scheduler::TasksScheduler,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub databases_layer_entry: DatabasesLayerEntry,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            databases_layer_entry: DatabasesLayerEntry::new(&config).await?,
        })
    }
}
