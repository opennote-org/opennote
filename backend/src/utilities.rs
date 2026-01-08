use std::sync::Arc;

use actix_web::web;
use qdrant_client::Qdrant;
use tokio::sync::{Mutex, RwLock};

use crate::{
    app_state::AppState, backup::archieve::ArchievesStorage, configurations::system::Config, identities::storage::UserInformationStorage, metadata_storage::MetadataStorage, tasks_scheduler::TasksScheduler
};

pub async fn acquire_data(
    data: &web::Data<RwLock<AppState>>,
) -> (
    String,
    Qdrant,
    Arc<Mutex<MetadataStorage>>,
    Arc<Mutex<TasksScheduler>>,
    Config,
    Arc<Mutex<UserInformationStorage>>,
    Arc<Mutex<ArchievesStorage>>,
) {
    let (
        index_name,
        db_client,
        metadata_storage,
        tasks_scheduler,
        config,
        user_information_storage,
        archieve_storage,
    ) = {
        let state = data.read().await;
        (
            state.config.database.index.clone(),
            state.database.get_client().clone(),
            state.metadata_storage.clone(),
            state.tasks_scheduler.clone(),
            state.config.clone(),
            state.user_information_storage.clone(),
            state.archieve_storage.clone(),
        )
    };
    (
        index_name,
        db_client,
        metadata_storage,
        tasks_scheduler,
        config,
        user_information_storage,
        archieve_storage,
    )
}
