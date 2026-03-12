use std::sync::Arc;

use local_embedded::LocalEmbedder;
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
    // TODO: move heavy-weight embedder work to dedicated thread when needed
    pub local_embedder: Option<Arc<LocalEmbedder>>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let embedder_config = &config.embedder;

        let local_embedder = match embedder_config.provider.trim() {
            "local" => Some(Arc::new(LocalEmbedder::new(&embedder_config.model)?)),
            _ => None,
        };

        Ok(Self {
            config: config.clone(),
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            databases_layer_entry: DatabasesLayerEntry::new(&config).await?,
            local_embedder,
        })
    }
}
