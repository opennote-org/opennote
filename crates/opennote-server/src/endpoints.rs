use actix_web::{
    HttpResponse,
    web::{Bytes, Data},
};
use anyhow::anyhow;

use opennote_bootstrap::ApplicationBootStrap;
use opennote_core_logics::{
    block::{create_blocks, delete_blocks, read_blocks, update_blocks},
    search::{search_by_keyword, search_by_semantics},
};
use opennote_data::search::models::RawSearchResult;
use opennote_models::{
    configurations::search::SupportedSearchMethod,
    server::{
        requests::{
            CreateBlocksInWorkspaceRequest, DeleteBlocksInWorkspaceRequest,
            ReadBlocksInWorkspaceRequest, SearchBlocksInWorkspaceRequest,
            UpdateBlocksInWorkspaceRequest, decrypt_request,
        },
        responses::{create_bad_response, create_base_response},
    },
};

/// Use this endpoint to retrieve blocks in this workspace
pub async fn read_workspace_blocks(
    data: Data<ApplicationBootStrap>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request: ReadBlocksInWorkspaceRequest =
        match decrypt_request(request, &configurations.system.server.shared_key) {
            Ok(req) => req,
            Err(e) => return create_bad_response(format!("Failed to decrypt request: {}", e)),
        };

    create_base_response(
        read_blocks(&data.databases, &request.block_query, request.has_vector).await,
        &configurations.system.server.shared_key,
    )
}

/// It will create one new block with a default title payload.
pub async fn create_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request: CreateBlocksInWorkspaceRequest =
        match decrypt_request(request, &configurations.system.server.shared_key) {
            Ok(req) => req,
            Err(e) => return create_bad_response(format!("Failed to decrypt request: {}", e)),
        };

    create_base_response(
        create_blocks(
            &configurations.system.vector_database,
            &data.databases,
            request.blocks,
        )
        .await,
        &configurations.system.server.shared_key,
    )
}

/// Delete n blocks specified by their ids.
/// This is a normal task that will only show up in the notification center on finish.
pub async fn delete_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request: DeleteBlocksInWorkspaceRequest =
        match decrypt_request(request, &configurations.system.server.shared_key) {
            Ok(req) => req,
            Err(e) => return create_bad_response(format!("Failed to decrypt request: {}", e)),
        };

    create_base_response(
        delete_blocks(
            &data.databases,
            &configurations.system.vector_database,
            request.block_ids,
        )
        .await,
        &configurations.system.server.shared_key,
    )
}

/// Update n blocks supplied in the parameter
pub async fn update_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request: UpdateBlocksInWorkspaceRequest =
        match decrypt_request(request, &configurations.system.server.shared_key) {
            Ok(req) => req,
            Err(e) => return create_bad_response(format!("Failed to decrypt request: {}", e)),
        };

    create_base_response(
        update_blocks(
            &configurations.system.vector_database,
            &data.databases,
            request.blocks,
        )
        .await,
        &configurations.system.server.shared_key,
    )
}

pub async fn search_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Bytes,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    let request: SearchBlocksInWorkspaceRequest =
        match decrypt_request(request, &configurations.system.server.shared_key) {
            Ok(req) => req,
            Err(e) => return create_bad_response(format!("Failed to decrypt request: {}", e)),
        };

    let results = match request.search_method {
        SupportedSearchMethod::Keyword => {
            if let Some(query) = request.query {
                search_by_keyword(&data.databases, request.block_ids, &query, request.top_n).await
            } else {
                return create_base_response::<Vec<RawSearchResult>>(
                    Err(anyhow!("No query found for the search")),
                    &configurations.system.server.shared_key,
                );
            }
        }
        SupportedSearchMethod::Semantic => {
            if let Some(query) = request.query_vector {
                search_by_semantics(&data.databases, request.block_ids, &query, request.top_n).await
            } else {
                return create_base_response::<Vec<RawSearchResult>>(
                    Err(anyhow!("No query found for the search")),
                    &configurations.system.server.shared_key,
                );
            }
        }
    };

    create_base_response(results, &configurations.system.server.shared_key)
}
