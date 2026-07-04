use actix_web::{
    HttpResponse,
    web::{Data, Json},
};

use opennote_bootstrap::ApplicationBootStrap;
use opennote_core_logics::{
    block::{create_blocks, delete_blocks, read_blocks, update_blocks},
    search::{search_by_keyword, search_by_semantics},
};
use opennote_data::database::enums::BlockQuery;
use opennote_models::{
    configurations::search::SupportedSearchMethod,
    server::{
        requests::{
            CreateBlocksInWorkspaceRequest, DeleteBlocksInWorkspaceRequest,
            SearchBlocksInWorkspaceRequest, UpdateBlocksInWorkspaceRequest,
        },
        responses::{BaseResponse, create_base_response},
    },
};

/// Use this endpoint to retrieve all blocks in this workspace
pub async fn read_workspace_blocks(data: Data<ApplicationBootStrap>) -> HttpResponse {
    create_base_response(read_blocks(&data.databases, &BlockQuery::All).await)
}

/// It will create one new block with a default title payload.
pub async fn create_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Json<CreateBlocksInWorkspaceRequest>,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    create_base_response(
        create_blocks(
            &configurations.system.vector_database,
            &data.databases,
            request.0.blocks,
        )
        .await,
    )
}

/// Delete n blocks specified by their ids.
/// This is a normal task that will only show up in the notification center on finish.
pub async fn delete_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Json<DeleteBlocksInWorkspaceRequest>,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    create_base_response(
        delete_blocks(
            &data.databases,
            &configurations.system.vector_database,
            request.0.block_ids,
        )
        .await,
    )
}

/// Update n blocks supplied in the parameter
pub async fn update_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Json<UpdateBlocksInWorkspaceRequest>,
) -> HttpResponse {
    let configurations = data.configurations.lock().await;

    create_base_response(
        update_blocks(
            &configurations.system.vector_database,
            &data.databases,
            request.0.blocks,
        )
        .await,
    )
}

pub async fn search_blocks_in_workspace(
    data: Data<ApplicationBootStrap>,
    request: Json<SearchBlocksInWorkspaceRequest>,
) -> HttpResponse {
    let results = match request.0.search_method {
        SupportedSearchMethod::Keyword => {
            if let Some(query) = request.0.query {
                search_by_keyword(
                    &data.databases,
                    request.0.block_ids,
                    &query,
                    request.0.top_n,
                )
                .await
            } else {
                return HttpResponse::Ok().json(BaseResponse {
                    status: false,
                    message: Some("No query found for the search".to_string()),
                    data: None,
                });
            }
        }
        SupportedSearchMethod::Semantic => {
            if let Some(query) = request.0.query_vector {
                search_by_semantics(
                    &data.databases,
                    request.0.block_ids,
                    &query,
                    request.0.top_n,
                )
                .await
            } else {
                return HttpResponse::Ok().json(BaseResponse {
                    status: false,
                    message: Some("No query found for the search".to_string()),
                    data: None,
                });
            }
        }
    };

    create_base_response(results)
}
