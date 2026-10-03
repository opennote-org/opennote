use std::sync::RwLock;

use actix_web::{
    HttpResponse,
    web::{Bytes, Data},
};
use anyhow::anyhow;

use opennote_core_logics::{
    block::{create_blocks, delete_blocks, read_blocks, update_blocks},
    bootstraps::server::ServerBootstrap,
    search::{search_by_keyword, search_by_semantics},
};
use opennote_models::{
    configurations::fields::search::SupportedSearchMethod,
    query::BlockQuery,
    search::RawSearchResult,
    server::{
        requests::{
            CreateBlocksInWorkspaceRequest, DeleteBlocksInWorkspaceRequest,
            ReadBlocksInWorkspaceRequest, RequestReindexBlocksRequest,
            SearchBlocksInWorkspaceRequest, SendReindexedBlocksRequest,
            UpdateBlocksInWorkspaceRequest,
        },
        responses::{
            create_bad_response, create_base_response, create_error_response,
            reindex::{RequestReindexBlocksResponse, SendReindexedBlocksResponse},
        },
    },
};

use crate::{helpers::perform_pre_request_validations, reindex::ReindexSessionManager};

/// Use this endpoint to retrieve blocks in this workspace
pub async fn read_workspace_blocks(data: Data<ServerBootstrap>, request: Bytes) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request = match perform_pre_request_validations::<ReadBlocksInWorkspaceRequest>(
        &configurations,
        None,
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    tracing::info!(
        "Read workspace with the following parameters: {:?}",
        &request
    );

    create_base_response(
        read_blocks(
            &data.databases,
            &request.block_query,
            request.has_vector,
            request.has_payload,
        )
        .await,
        &configurations.shared_key,
    )
}

/// It will create one new block with a default title payload.
pub async fn create_blocks_in_workspace(
    data: Data<ServerBootstrap>,
    reindex_session_manager: Data<RwLock<ReindexSessionManager>>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request = match perform_pre_request_validations::<CreateBlocksInWorkspaceRequest>(
        &configurations,
        Some(reindex_session_manager),
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    tracing::info!(
        "Create blocks with the following parameters: {:?}",
        &request
    );

    create_base_response(
        create_blocks(
            &configurations.system.vector_database,
            &data.databases,
            request.blocks,
        )
        .await,
        &configurations.shared_key,
    )
}

/// Delete n blocks specified by their ids.
/// This is a normal task that will only show up in the notification center on finish.
pub async fn delete_blocks_in_workspace(
    data: Data<ServerBootstrap>,
    reindex_session_manager: Data<RwLock<ReindexSessionManager>>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request = match perform_pre_request_validations::<DeleteBlocksInWorkspaceRequest>(
        &configurations,
        Some(reindex_session_manager),
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    tracing::info!(
        "Delete blocks with the following parameters: {:?}",
        &request
    );

    create_base_response(
        delete_blocks(
            &data.databases,
            &configurations.system.vector_database,
            request.block_ids,
        )
        .await,
        &configurations.shared_key,
    )
}

/// Update n blocks supplied in the parameter
pub async fn update_blocks_in_workspace(
    data: Data<ServerBootstrap>,
    reindex_session_manager: Data<RwLock<ReindexSessionManager>>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request = match perform_pre_request_validations::<UpdateBlocksInWorkspaceRequest>(
        &configurations,
        Some(reindex_session_manager),
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    tracing::info!(
        "Update blocks with the following parameters: {:?}",
        &request
    );

    create_base_response(
        update_blocks(
            &configurations.system.vector_database,
            &data.databases,
            request.blocks,
        )
        .await,
        &configurations.shared_key,
    )
}

pub async fn search_blocks_in_workspace(
    data: Data<ServerBootstrap>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request = match perform_pre_request_validations::<SearchBlocksInWorkspaceRequest>(
        &configurations,
        None,
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    tracing::info!(
        "Search blocks with the following parameters: {:?}",
        &request
    );

    let results = match request.search_method {
        SupportedSearchMethod::Keyword => {
            if let Some(query) = request.query {
                search_by_keyword(&data.databases, request.block_ids, &query, request.top_n).await
            } else {
                return create_base_response::<Vec<RawSearchResult>>(
                    Err(anyhow!("No query found for the search")),
                    &configurations.shared_key,
                );
            }
        }
        SupportedSearchMethod::Semantic => {
            if let Some(query) = request.query_vector {
                search_by_semantics(&data.databases, request.block_ids, &query, request.top_n).await
            } else {
                return create_base_response::<Vec<RawSearchResult>>(
                    Err(anyhow!("No query found for the search")),
                    &configurations.shared_key,
                );
            }
        }
    };

    create_base_response(results, &configurations.shared_key)
}

/// General flow:
/// 1. Desktop sends a request to the server for reindexing
/// 2. Server sends its blocks in turns to the desktop
///
/// 3. Desktop returns embedded blocks in turns
/// 4. Server store the blocks
pub async fn request_reindex_workspace(
    data: Data<ServerBootstrap>,
    reindex_session_manager: Data<RwLock<ReindexSessionManager>>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let _request = match perform_pre_request_validations::<RequestReindexBlocksRequest>(
        &configurations,
        None, // Will handle this check later
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    // Get all blocks from the database.
    // This will also lock the blocks up to the point when this endpoint is called.
    let blocks = match read_blocks(&data.databases, &BlockQuery::All, false, true).await {
        Ok(blocks) => blocks,
        Err(e) => return create_error_response(e.to_string(), &configurations.shared_key),
    };

    // Create a new session in the state
    let mut reindex_session_manager = reindex_session_manager.write().unwrap();
    reindex_session_manager.new_session(blocks);

    tracing::info!("Reindex request received");

    // Return the session id
    create_base_response(
        Ok(RequestReindexBlocksResponse {}),
        &configurations.shared_key,
    )
}

pub async fn send_reindexed_blocks_to_workspace(
    data: Data<ServerBootstrap>,
    reindex_session_manager: Data<RwLock<ReindexSessionManager>>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request = match perform_pre_request_validations::<SendReindexedBlocksRequest>(
        &configurations,
        None, // Will handle this check later
        request,
    ) {
        Ok(value) => value,
        Err(value) => return value,
    };

    tracing::info!("{} vectorized blocks received", request.blocks.len());

    let mut reindex_session_manager = reindex_session_manager.write().unwrap();

    // Record the received the blocks into the state
    let session = match reindex_session_manager.get_session_mut() {
        Some(session) => session,
        None => return create_bad_response("No active reindex session".to_string()),
    };

    session.add_blocks(request.blocks);

    let has_session_finished = session.has_session_finished();

    // Proceed to overwrite when the state is complete.
    // At this point, we assume the user had made a backup copy of the original databases.
    if has_session_finished {
        let blocks = reindex_session_manager
            .get_session_mut()
            .expect("Should have a session")
            .get_session_contents();

        // Since we had assumed the user should have made a backup copy at this point,
        // we terminate the session without leaving it stale.
        reindex_session_manager.end_session();

        // Delete existing blocks
        match delete_blocks(
            &data.databases,
            &configurations.system.vector_database,
            blocks.iter().map(|item| item.id).collect(),
        )
        .await
        {
            Ok(_) => {}
            Err(e) => return create_error_response(e.to_string(), &configurations.shared_key),
        }

        // Write the blocks
        match create_blocks(
            &configurations.system.vector_database,
            &data.databases,
            blocks,
        )
        .await
        {
            Ok(_) => {}
            Err(e) => return create_error_response(e.to_string(), &configurations.shared_key),
        }

        // Return early
        return create_base_response(
            Ok(SendReindexedBlocksResponse {
                blocks: Vec::new(),
                finished: true,
            }),
            &configurations.shared_key,
        );
    }

    // Return the next batch of blocks to reindex
    let unfinished_blocks = session.take_unfinished_blocks(
        configurations
            .system
            .vector_database
            .reindex_batch_size
            .into(),
    );

    tracing::info!("{} unfinished blocks sent", unfinished_blocks.len());

    create_base_response(
        Ok(SendReindexedBlocksResponse {
            blocks: unfinished_blocks,
            finished: false,
        }),
        &configurations.shared_key,
    )
}
