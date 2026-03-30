use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::configurations::system::Config;
use opennote_tasks_scheduler::TasksScheduler;

#[derive(Clone)]
pub struct ApplicationBootStrap {
    pub config: Config,
    pub databases_layer_entry: Databases,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
    pub embedder_entry: Arc<EmbedderEntry>,
}

impl ApplicationBootStrap {
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            databases_layer_entry: Databases::new(&config).await?,
            embedder_entry: Arc::new(EmbedderEntry::new(&config).await?),
        })
    }
}
