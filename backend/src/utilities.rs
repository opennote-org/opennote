use std::sync::Arc;

use actix_web::web;
use tokio::sync::{Mutex, RwLock};

use crate::{
    app_state::AppState, backup::storage::BackupsStorage, configurations::system::Config,
    identities::storage::IdentitiesStorage, metadata_storage::MetadataStorage,
    tasks_scheduler::TasksScheduler, vector_database::traits::VectorDatabase,
};

pub async fn acquire_data(
    data: &web::Data<RwLock<AppState>>,
) -> (
    Arc<dyn VectorDatabase>,
    Arc<Mutex<MetadataStorage>>,
    Arc<Mutex<TasksScheduler>>,
    Config,
    Arc<Mutex<IdentitiesStorage>>,
    Arc<Mutex<BackupsStorage>>,
) {
    let (
        vector_database,
        metadata_storage,
        tasks_scheduler,
        config,
        identities_storage,
        backups_storage,
    ) = {
        let state = data.read().await;
        (
            state.database.clone(),
            state.metadata_storage.clone(),
            state.tasks_scheduler.clone(),
            state.config.clone(),
            state.identities_storage.clone(),
            state.backups_storage.clone(),
        )
    };
    (
        vector_database,
        metadata_storage,
        tasks_scheduler,
        config,
        identities_storage,
        backups_storage,
    )
}
