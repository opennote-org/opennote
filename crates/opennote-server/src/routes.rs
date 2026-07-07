use actix_web::{Scope, web};
use opennote_models::constants::{
    CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT, DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT,
    READ_WORKSPACE_BLOCKS_ENDPOINT, ROOT_ENDPOINT, SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT,
    UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
};

use crate::endpoints::{
    create_blocks_in_workspace, delete_blocks_in_workspace, read_workspace_blocks,
    search_blocks_in_workspace, update_blocks_in_workspace,
};

pub fn configure_routes() -> Scope {
    web::scope(ROOT_ENDPOINT)
        .route(
            READ_WORKSPACE_BLOCKS_ENDPOINT,
            web::get().to(read_workspace_blocks),
        )
        .route(
            CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
            web::post().to(create_blocks_in_workspace),
        )
        .route(
            DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT,
            web::delete().to(delete_blocks_in_workspace),
        )
        .route(
            UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
            web::put().to(update_blocks_in_workspace),
        )
        .route(
            SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT,
            web::post().to(search_blocks_in_workspace),
        )
}
