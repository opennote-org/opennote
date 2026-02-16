use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    backup::storage::BackupsStorage,
    configurations::system::Config,
    database::sqlite::SQLiteDatabase,
    identities::storage::IdentitiesStorage,
    tasks_scheduler::TasksScheduler,
    traits::LoadAndSave,
    vector_database::{shared::create_vector_database, traits::VectorDatabase},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub tasks_scheduler: Arc<Mutex<TasksScheduler>>,
    pub vector_database: Arc<dyn VectorDatabase>,
    pub backups_storage: Arc<Mutex<BackupsStorage>>,
    pub identities_storage: Arc<Mutex<IdentitiesStorage>>,
    pub database: Arc<SQLiteDatabase>,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config_clone = config.clone();
        let vector_database = create_vector_database(&config).await?;

        let database: SQLiteDatabase = SQLiteDatabase::new(&config.database.connection_url).await?;
        database.migrate(&config.metadata_storage.path).await?;

        Ok(Self {
            config,
            tasks_scheduler: Arc::new(Mutex::new(TasksScheduler::new())),
            vector_database,
            backups_storage: Arc::new(Mutex::new(BackupsStorage::load(
                &config_clone.backups_storage.path,
            )?)),
            identities_storage: Arc::new(Mutex::new(IdentitiesStorage::load(
                &config_clone.identities_storage.path,
            )?)),
            database: Arc::new(database),
        })
    }
}
