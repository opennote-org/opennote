use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::configurations::Configurations;
use opennote_tasks_scheduler::TasksScheduler;

#[derive(Clone)]
pub struct ApplicationBootStrap {
    pub configurations: Arc<Configurations>,
    pub databases: Databases,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
    pub embedders: Arc<EmbedderEntry>,
}

impl ApplicationBootStrap {
    pub async fn new(configurations: Configurations) -> Result<Self> {
        Ok(Self {
            configurations: Arc::new(configurations.clone()),
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            databases: Databases::new(&configurations.system).await?,
            embedders: Arc::new(EmbedderEntry::new(&configurations.system).await?),
        })
    }
}
