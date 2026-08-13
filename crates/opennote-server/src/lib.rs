use actix_web::http::header::AUTHORIZATION;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_encrypt::shared_key::SharedKey;
use uuid::Uuid;

use opennote_models::{
    block::Block,
    configurations::search::SupportedSearchMethod,
    constants::{
        CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT, DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT,
        READ_WORKSPACE_BLOCKS_ENDPOINT, ROOT_ENDPOINT, SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT,
        UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT,
    },
    query::BlockQuery,
    search::RawSearchResult,
    server::{
        requests::{
            CreateBlocksInWorkspaceRequest, DeleteBlocksInWorkspaceRequest,
            ReadBlocksInWorkspaceRequest, SearchBlocksInWorkspaceRequest,
            UpdateBlocksInWorkspaceRequest, create_request,
        },
        responses::parse_base_response,
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

pub async fn read_remote_server_blocks(
    client: &Client,
    base_url: &str,
    password: &str,
    shared_key: &SharedKey,
    filter: &BlockQuery,
    has_vector: bool,
    has_payload: bool,
) -> Result<Vec<Block>> {
    let payload = ReadBlocksInWorkspaceRequest {
        block_query: filter.to_owned(),
        has_vector,
        has_payload,
    };
    let body = create_request(payload, shared_key)?.serialize();

    let response = client
        .post(build_url(base_url, READ_WORKSPACE_BLOCKS_ENDPOINT))
        .header(AUTHORIZATION.as_str(), password)
        .body(body)
        .send()
        .await
        .context("Failed to send read request")?;

    parse_base_response(response, &shared_key).await
}

pub async fn create_remote_server_blocks(
    client: &Client,
    base_url: &str,
    password: &str,
    blocks: Vec<Block>,
    shared_key: &SharedKey,
) -> Result<Vec<Block>> {
    let payload = CreateBlocksInWorkspaceRequest { blocks };
    let body = create_request(payload, shared_key)?.serialize();

    let response = client
        .post(build_url(base_url, CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .header(AUTHORIZATION.as_str(), password)
        .body(body)
        .send()
        .await
        .context("Failed to send create request")?;

    parse_base_response(response, shared_key).await
}

pub async fn delete_remote_server_blocks(
    client: &Client,
    base_url: &str,
    password: &str,
    block_ids: Vec<Uuid>,
    shared_key: &SharedKey,
) -> Result<()> {
    let payload = DeleteBlocksInWorkspaceRequest { block_ids };
    let body = create_request(payload, shared_key)?.serialize();

    let response = client
        .delete(build_url(base_url, DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .header(AUTHORIZATION.as_str(), password)
        .body(body)
        .send()
        .await
        .context("Failed to send delete request")?;

    parse_base_response(response, shared_key).await
}

pub async fn update_remote_server_blocks(
    client: &Client,
    base_url: &str,
    password: &str,
    blocks: Vec<Block>,
    shared_key: &SharedKey,
) -> Result<()> {
    let payload = UpdateBlocksInWorkspaceRequest { blocks };
    let body = create_request(payload, shared_key)?.serialize();

    let response = client
        .put(build_url(base_url, UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .header(AUTHORIZATION.as_str(), password)
        .body(body)
        .send()
        .await
        .context("Failed to send update request")?;

    parse_base_response(response, shared_key).await
}

pub async fn search_remote_server_blocks(
    client: &Client,
    base_url: &str,
    password: &str,
    search_method: SupportedSearchMethod,
    block_ids: Vec<Uuid>,
    query: Option<String>,
    query_vector: Option<Vec<f32>>,
    top_n: usize,
    shared_key: &SharedKey,
) -> Result<Vec<RawSearchResult>> {
    let payload = SearchBlocksInWorkspaceRequest {
        search_method,
        block_ids,
        query,
        query_vector,
        top_n,
    };
    let body = create_request(payload, shared_key)?.serialize();

    let response = match client
        .post(build_url(base_url, SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT))
        .header(AUTHORIZATION.as_str(), password)
        .body(body)
        .send()
        .await
    {
        Ok(result) => result,
        // Server connection status should be indicated somewhere else.
        // A server error should not block the entire search opearation.
        Err(_) => return Ok(Vec::new()),
    };

    parse_base_response(response, shared_key).await
}
