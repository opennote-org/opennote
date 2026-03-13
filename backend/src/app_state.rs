use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    configurations::system::Config, databases::entry::DatabasesLayerEntry,
    embedders::entry::EmbedderEntry, tasks_scheduler::TasksScheduler,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub databases_layer_entry: DatabasesLayerEntry,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
    pub embedder_entry: Arc<EmbedderEntry>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            databases_layer_entry: DatabasesLayerEntry::new(&config).await?,
            embedder_entry: Arc::new(EmbedderEntry::new(&config).await?),
        })
    }
}
