use anyhow::{Context, Error, Result, anyhow};
use async_trait::async_trait;
use futures::future::{join_all, try_join_all};
use gpui_kit::{App, AppContext, Global, SharedString};
use regex::Regex;
use uuid::Uuid;

use opennote_core_logics::{
    bootstraps::desktop::DesktopBootstrap, helpers::run_async_code,
    payload::convert_string_to_payloads,
};
use opennote_embedder::vectorization::{send_vectorization, vectorize};
use opennote_mcp_server::{
    requests::{
        MCPCreateBlockRequest, MCPDeleteBlocksRequest, MCPReadBlockDetailsRequest,
        MCPReadBlocksRequest, MCPSearchRequest, MCPUpdateBlocksInsertionOperation,
        MCPUpdateBlocksRequest,
    },
    run_mcp_server,
    traits::OpenNoteMCPServiceImplementation,
};
use opennote_models::{
    configurations::fields::search::SupportedSearchMethod, payload::create_query,
    query::BlockQuery, traits::summarization::summarize_all,
};

use crate::globals::{
    actions::{
        block::{build_block, get_blocks_from_servers},
        route_helpers::{
            route_create_blocks, route_delete_blocks, route_read_blocks, route_update_blocks,
        },
        search::{SearchResult, search},
    },
    bootstrap::helpers::get_bootstrap,
    states::{helpers::get_states, server_registry::ServerRegistry},
};

pub struct DesktopMCPServer {
    server_registry: ServerRegistry,
    bootstrap: DesktopBootstrap,
}

impl Global for DesktopMCPServer {}

impl DesktopMCPServer {
    pub fn init(cx: &mut App) -> Result<()> {
        let bootstrap = get_bootstrap(cx);
        let configurations = run_async_code(async {
            bootstrap
                .0
                .configurations
                .lock()
                .await
                .user
                .mcp_server
                .clone()
        });

        if !configurations.enabled {
            return Ok(());
        }

        let states = get_states(cx);

        let mcp_server = DesktopMCPServer::new(states.get_server_registry(), bootstrap.0.clone());

        let server = run_mcp_server(
            &configurations.get_mcp_server_address(),
            configurations.workers,
            std::sync::Arc::new(mcp_server),
        )?;

        cx.background_spawn(async {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async { server.await.unwrap() })
        })
        .detach();

        Ok(())
    }

    pub fn new(server_registry: ServerRegistry, bootstrap: DesktopBootstrap) -> Self {
        Self {
            server_registry,
            bootstrap,
        }
    }

    fn parse_block_ids(block_ids: Vec<String>) -> Result<Vec<Uuid>> {
        block_ids
            .into_iter()
            .map(|block_id| {
                Uuid::parse_str(&block_id).with_context(|| format!("Invalid block ID `{block_id}`"))
            })
            .collect()
    }
}

#[async_trait]
impl OpenNoteMCPServiceImplementation for DesktopMCPServer {
    async fn search(&self, request: MCPSearchRequest) -> Result<String> {
        let MCPSearchRequest {
            search_method,
            block_ids,
            query,
            top_n,
        } = request;

        let configurations = self.bootstrap.configurations.lock().await;

        configurations
            .user
            .mcp_server
            .validate_access(false, true)?;

        let block_ids = Self::parse_block_ids(block_ids)?;

        let query_vector = match search_method {
            SupportedSearchMethod::Keyword => None,
            SupportedSearchMethod::Semantic => {
                let vectorized_query =
                    send_vectorization(vec![create_query(&query)], &self.bootstrap.embedders)
                        .await?
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow!("Embedder returned no query vector"))?;

                Some(vectorized_query.vector)
            }
        };

        let servers = self.server_registry.get_servers_connections();

        let results: Vec<SearchResult> = try_join_all(
            servers
                .iter()
                .filter(|(server_name, _server_states)| {
                    configurations
                        .user
                        .mcp_server
                        .validate_scopes(server_name.to_owned().to_string())
                })
                .map(async |(server_name, server_states)| {
                    let mut results = search(
                        server_name,
                        server_states,
                        &self.bootstrap.databases,
                        block_ids.clone(),
                        search_method,
                        configurations.system.clone(),
                        Some(query.clone()),
                        query_vector.clone(),
                        top_n,
                    )
                    .await?;

                    results.sort_by(|left, right| right.raw.score.total_cmp(&left.raw.score));
                    results.truncate(top_n);

                    Ok::<Vec<SearchResult>, Error>(results)
                }),
        )
        .await?
        .into_iter()
        .flatten()
        .collect();

        Ok(summarize_all(results))
    }

    async fn read_blocks(&self, request: MCPReadBlocksRequest) -> Result<String> {
        let block_ids = Self::parse_block_ids(request.block_ids)?;

        let configurations = self.bootstrap.configurations.lock().await.clone();

        configurations
            .user
            .mcp_server
            .validate_access(false, true)?;

        let filter = match block_ids.is_empty() {
            true => BlockQuery::All,
            false => BlockQuery::ByIds(block_ids),
        };

        let servers = self.server_registry.get_servers_connections();

        let blocks = get_blocks_from_servers(
            filter,
            |server_name, _server_states| {
                configurations
                    .user
                    .mcp_server
                    .validate_scopes(server_name.to_string())
            },
            servers,
            self.bootstrap.databases.clone(),
            configurations.system.clone(),
            false,
            false,
        )
        .await?;

        Ok(summarize_all(blocks))
    }

    async fn read_block_details(&self, request: MCPReadBlockDetailsRequest) -> Result<String> {
        let block_ids = Self::parse_block_ids(vec![request.block_id.clone()])?;

        let configurations = self.bootstrap.configurations.lock().await.clone();

        configurations
            .user
            .mcp_server
            .validate_access(false, true)?;

        let filter = BlockQuery::ByIds(block_ids);

        let servers = self.server_registry.get_servers_connections();

        let blocks = get_blocks_from_servers(
            filter,
            |server_name, _server_states| {
                configurations
                    .user
                    .mcp_server
                    .validate_scopes(server_name.to_string())
            },
            servers,
            self.bootstrap.databases.clone(),
            configurations.system.clone(),
            false,
            true,
        )
        .await?;

        match blocks.get(0) {
            Some(block) => Ok(block.get_text_content()),
            None => Err(anyhow!("Block {} missing", &request.block_id)),
        }
    }

    /// TODO:
    /// - Add a rwlock to the database
    async fn update_block(&self, request: MCPUpdateBlocksRequest) -> Result<()> {
        // Verify if this block blongs to the allowed scope
        let configurations = self.bootstrap.configurations.lock().await.clone();
        let mcp_configurations = &configurations.user.mcp_server;

        mcp_configurations.validate_access(true, false)?;

        let servers = self.server_registry.get_servers_connections();
        let block_id = Self::parse_block_ids(vec![request.block_id])?[0];

        let mut tasks = Vec::new();

        for (server_name, server_states) in servers {
            if mcp_configurations.validate_scopes(server_name.to_string()) {
                let system_configurations = configurations.system.clone();

                tasks.push(async move {
                    let server_name = server_name.clone();
                    let server_states = server_states.clone();

                    (
                        route_read_blocks(
                            &server_name,
                            &server_states,
                            &self.bootstrap.databases,
                            &BlockQuery::All,
                            false,
                            false,
                            system_configurations,
                        )
                        .await,
                        server_name,
                        server_states,
                    )
                });
            }
        }

        // Get a complete block
        let mut block = None;
        let (mut server_name, mut server_states) = (None, None);

        for result in join_all(tasks).await {
            let results = result.0?;
            let found_block = results.iter().find(|block| block.id == block_id);

            if let Some(found_block) = found_block {
                server_name = Some(result.1.clone());
                server_states = Some(result.2.clone());

                block = route_read_blocks(
                    &result.1,
                    &result.2,
                    &self.bootstrap.databases,
                    &BlockQuery::ByIds(vec![found_block.id]),
                    false,
                    true,
                    configurations.system.clone(),
                )
                .await?
                .pop();
            }
        }

        // Get block content
        let Some(mut block) = block else {
            return Err(anyhow!("Missing block: {}", block_id));
        };
        let mut content = block.get_text_content();

        // Perform insertion
        if let Some(insertion) = request.insert {
            match insertion.operation {
                MCPUpdateBlocksInsertionOperation::Start => content.insert_str(0, &insertion.text),
                MCPUpdateBlocksInsertionOperation::End => content.push_str(&insertion.text),
            }
        }

        // Perform replacement
        if let Some(replacement) = request.replace {
            let regex = Regex::new(&replacement.regex)?;
            content = regex.replace(&content, &replacement.text).to_string();
        }

        let payloads = convert_string_to_payloads(
            block_id,
            Some(configurations.user.search.document_chunk_size),
            content,
            None,
        )?;

        // Update the block
        match (server_name, server_states) {
            (Some(server_name), Some(server_states)) => {
                block.payloads = vectorize(
                    &self.bootstrap.embedders,
                    &configurations.system.embedder,
                    payloads,
                )
                .await?;

                route_update_blocks(
                    &server_name,
                    &server_states,
                    &self.bootstrap.databases,
                    configurations.system,
                    vec![block],
                )
                .await?;
            }
            _ => return Ok(()),
        }

        Ok(())
    }

    async fn create_block(&self, request: MCPCreateBlockRequest) -> Result<()> {
        let configurations = self.bootstrap.configurations.lock().await;

        configurations
            .user
            .mcp_server
            .validate_access(true, false)?;

        let servers = self.server_registry.get_servers_connections();
        let server_name: SharedString = request.scope.clone().into();

        if !configurations
            .user
            .mcp_server
            .validate_scopes(server_name.clone().into())
        {
            return Err(anyhow!("Server {} is not allowed for access", &server_name));
        }

        let parent_block_id = match request.parent_block_id {
            Some(result) => Some(Self::parse_block_ids(vec![result])?[0]),
            None => None,
        };

        let block = build_block(
            parent_block_id,
            request.title,
            &self.bootstrap.embedders,
            None,
            None,
        )
        .await?;

        match servers.get(&server_name) {
            Some(server_states) => {
                route_create_blocks(
                    &server_name,
                    server_states,
                    &self.bootstrap.databases,
                    configurations.system.clone(),
                    vec![block],
                )
                .await?
            }
            None => return Err(anyhow!("Missing server: {}", &server_name)),
        };

        Ok(())
    }

    async fn delete_blocks(&self, request: MCPDeleteBlocksRequest) -> Result<()> {
        let configurations = self.bootstrap.configurations.lock().await;

        configurations
            .user
            .mcp_server
            .validate_access(true, false)?;

        let servers = self.server_registry.get_servers_connections();
        let server_name: SharedString = request.scope.clone().into();
        let block_ids = Self::parse_block_ids(request.block_ids)?;

        if !configurations
            .user
            .mcp_server
            .validate_scopes(server_name.to_string())
        {
            return Err(anyhow!("Server {} is not allowed for access", server_name));
        }

        match servers.get(&server_name) {
            Some(server_states) => {
                route_delete_blocks(
                    &server_name,
                    server_states,
                    &self.bootstrap.databases,
                    configurations.system.clone(),
                    block_ids,
                )
                .await?
            }
            None => return Err(anyhow!("Missing server: {}", &server_name)),
        };

        Ok(())
    }
}
