use anyhow::{Context, Result, bail};
use reqwest::Client;
use uuid::Uuid;

use opennote_data::search::models::RawSearchResult;
use opennote_models::{
    block::Block,
    configurations::search::SupportedSearchMethod,
    constants::{
        CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT, DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT,
        READ_WORKSPACE_BLOCKS_ENDPOINT, ROOT_ENDPOINT, SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT,
        UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
    },
    server::{
        requests::{
            CreateBlocksInWorkspaceRequest, DeleteBlocksInWorkspaceRequest,
            SearchBlocksInWorkspaceRequest, UpdateBlocksInWorkspaceRequest,
        },
        responses::BaseResponse,
    },
};

fn build_url(base_url: &str, endpoint: &str) -> String {
    format!(
        "{}{}{}",
        base_url.trim_end_matches('/'),
        ROOT_ENDPOINT,
        endpoint
    )
}

async fn parse_response<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let base: BaseResponse = response
        .json()
        .await
        .context("Failed to deserialize BaseResponse")?;

    if base.status {
        serde_json::from_value(base.data.unwrap_or_default())
            .context("Failed to deserialize response data")
    } else {
        bail!(
            base.message
                .unwrap_or_else(|| "Unknown server error".to_string())
        )
    }
}

pub async fn read_remote_server_blocks(client: &Client, base_url: &str) -> Result<Vec<Block>> {
    let response = client
        .get(build_url(base_url, READ_WORKSPACE_BLOCKS_ENDPOINT))
        .send()
        .await
        .context("Failed to send read request")?;
    parse_response(response).await
}

pub async fn create_remote_server_blocks(
    client: &Client,
    base_url: &str,
    blocks: Vec<Block>,
) -> Result<Vec<Block>> {
    let response = client
        .post(build_url(base_url, CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .json(&CreateBlocksInWorkspaceRequest { blocks })
        .send()
        .await
        .context("Failed to send create request")?;
    parse_response(response).await
}

pub async fn delete_remote_server_blocks(
    client: &Client,
    base_url: &str,
    block_ids: Vec<Uuid>,
) -> Result<()> {
    let response = client
        .delete(build_url(base_url, DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .json(&DeleteBlocksInWorkspaceRequest { block_ids })
        .send()
        .await
        .context("Failed to send delete request")?;
    parse_response(response).await
}

pub async fn update_remote_server_blocks(
    client: &Client,
    base_url: &str,
    blocks: Vec<Block>,
) -> Result<()> {
    let response = client
        .put(build_url(base_url, UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .json(&UpdateBlocksInWorkspaceRequest { blocks })
        .send()
        .await
        .context("Failed to send update request")?;
    parse_response(response).await
}

pub async fn search_remote_server_blocks(
    client: &Client,
    base_url: &str,
    search_method: SupportedSearchMethod,
    block_ids: Vec<Uuid>,
    query: Option<String>,
    query_vector: Option<Vec<f32>>,
    top_n: usize,
) -> Result<Vec<RawSearchResult>> {
    let response = client
        .post(build_url(base_url, SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .json(&SearchBlocksInWorkspaceRequest {
            search_method,
            block_ids,
            query,
            query_vector,
            top_n,
        })
        .send()
        .await
        .context("Failed to send search request")?;
    parse_response(response).await
}
