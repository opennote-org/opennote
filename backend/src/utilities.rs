use std::sync::Arc;

use actix_web::web;
use tokio::sync::{Mutex, RwLock};

use crate::{
    app_state::AppState, backup::storage::BackupsStorage, configurations::system::Config,
    database::traits::Database, identities::storage::IdentitiesStorage,
    tasks_scheduler::TasksScheduler, vector_database::traits::VectorDatabase,
};

pub async fn acquire_data(
    data: &web::Data<RwLock<AppState>>,
) -> (
    Arc<dyn VectorDatabase>,
    Arc<Mutex<TasksScheduler>>,
    Config,
    Arc<Mutex<IdentitiesStorage>>,
    Arc<Mutex<BackupsStorage>>,
    Arc<dyn Database>,
) {
    let (vector_database, tasks_scheduler, config, identities_storage, backups_storage, database) = {
        let state = data.read().await;
        (
            state.vector_database.clone(),
            state.tasks_scheduler.clone(),
            state.config.clone(),
            state.identities_storage.clone(),
            state.backups_storage.clone(),
            state.database.clone(),
        )
    };
    (
        vector_database,
        tasks_scheduler,
        config,
        identities_storage,
        backups_storage,
        database,
    )
}
