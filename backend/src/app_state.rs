use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    backup::storage::BackupsStorage,
    configurations::system::Config,
    database::{shared::create_database, traits::database::Database},
    tasks_scheduler::TasksScheduler,
    traits::LoadAndSave,
    vector_database::{shared::create_vector_database, traits::VectorDatabase},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub database: Arc<dyn Database>,
    pub vector_database: Arc<dyn VectorDatabase>,
    pub backups_storage: Arc<Mutex<BackupsStorage>>,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config_clone = config.clone();
        let vector_database = create_vector_database(&config).await?;

        let database = create_database(&config).await?;
        
        if !database.is_database_exist().await {
            log::warn!("Database does not exist! Creating a new one...");
            database.migrate(
                &config.metadata_storage.path,
                &config.identities_storage.path,
                &vector_database
            ).await?;
        }
        log::info!("Database exists! Skip creating a new one...");
        
        Ok(Self {
            config,
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            database,
            vector_database,
            backups_storage: Arc::new(Mutex::new(BackupsStorage::load(
                &config_clone.backups_storage.path,
            )?)),
        })
    }
}
