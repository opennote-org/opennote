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
    pub embedders: Option<Arc<EmbedderEntry>>,
}

impl ApplicationBootStrap {
    pub async fn new(configurations: Configurations) -> Result<Self> {
        let embedders = match EmbedderEntry::new(&configurations.system).await {
            Ok(result) => Some(Arc::new(result)),
            Err(error) => {
                log::warn!("Error when loading an embedding model: {}", error);
                None
            },
        };

        Ok(Self {
            configurations: Arc::new(configurations.clone()),
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            databases: Databases::new(&configurations.system).await?,
            embedders,
        })
    }

    /// Reload an embedder model during  the runtime based on the lastest system configurations
    pub async fn reload_embedder(&mut self) -> Result<()> {
        self.embedders = Some(Arc::new(
            EmbedderEntry::new(&self.configurations.system).await?,
        ));

        Ok(())
    }
}
