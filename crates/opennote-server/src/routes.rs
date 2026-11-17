use actix_web::{Scope, web};
use opennote_models::constants::{
    BUFFER_ASSETS_FROM_SERVER_ENDPOINT, CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
    DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT, QUERY_SERVER_ASSETS_AVAILABILITY_ENDPOINT,
    READ_WORKSPACE_BLOCKS_ENDPOINT, REQUEST_REINDEX_WORKSPACE_ENDPOINT, ROOT_ENDPOINT,
    SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT, SEND_ASSETS_TO_SERVER_ENDPOINT,
    SEND_REINDEXED_BLOCKS_TO_WORKSPACE_ENDPOINT, UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
};

use crate::endpoints::{
    buffer_assets_from_server, create_blocks_in_workspace, delete_blocks_in_workspace,
    query_server_assets_availability, read_workspace_blocks, request_reindex_workspace,
    search_blocks_in_workspace, send_assets_to_server, send_reindexed_blocks_to_workspace,
    update_blocks_in_workspace,
};

pub fn configure_routes() -> Scope {
    web::scope(ROOT_ENDPOINT)
        .route(
            READ_WORKSPACE_BLOCKS_ENDPOINT,
            web::post().to(read_workspace_blocks),
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
        .route(
            REQUEST_REINDEX_WORKSPACE_ENDPOINT,
            web::post().to(request_reindex_workspace),
        )
        .route(
            SEND_REINDEXED_BLOCKS_TO_WORKSPACE_ENDPOINT,
            web::post().to(send_reindexed_blocks_to_workspace),
        )
        .route(
            SEND_ASSETS_TO_SERVER_ENDPOINT,
            web::post().to(send_assets_to_server),
        )
        .route(
            BUFFER_ASSETS_FROM_SERVER_ENDPOINT,
            web::post().to(buffer_assets_from_server),
        )
        .route(
            QUERY_SERVER_ASSETS_AVAILABILITY_ENDPOINT,
            web::post().to(query_server_assets_availability),
        )
}
