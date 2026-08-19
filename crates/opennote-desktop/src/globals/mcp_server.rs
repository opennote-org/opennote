use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use futures::future::try_join_all;
use gpui::{App, AppContext, Global};
use opennote_core_logics::helpers::run_async_code;
use uuid::Uuid;

use opennote_bootstrap::DesktopBootstrap;
use opennote_embedder::vectorization::send_vectorization;
use opennote_mcp_server::{
    requests::{MCPReadBlocksRequest, MCPSearchRequest},
    run_mcp_server,
    traits::OpenNoteMCPServiceImplementation,
};
use opennote_models::{
    block::Block, configurations::fields::search::SupportedSearchMethod, payload::create_query,
    query::BlockQuery,
};

use crate::globals::{
    actions::route_helpers::{route_read_blocks, route_search_blocks},
    bootstrap::GlobalApplicationBootStrap,
    states::{States, server_registry::ServerRegistry},
};

pub struct DesktopMCPServer {
    server_registry: ServerRegistry,
    bootstrap: DesktopBootstrap,
}

impl Global for DesktopMCPServer {}

impl DesktopMCPServer {
    pub fn init(cx: &mut App) {
        let bootstrap: &GlobalApplicationBootStrap = cx.global();
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
            return;
        }

        let states: &States = cx.global();

        let mcp_server = DesktopMCPServer::new(states.get_server_registry(), bootstrap.0.clone());

        let server = run_mcp_server(
            &configurations.get_mcp_server_address(),
            configurations.workers,
            std::sync::Arc::new(mcp_server),
        )
        .unwrap();

        cx.background_spawn(async {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async { server.await.unwrap() })
        })
        .detach();
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
    async fn search(&self, request: MCPSearchRequest) -> Result<Vec<Block>> {
        let MCPSearchRequest {
            search_method,
            block_ids,
            query,
            top_n,
        } = request;

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

        let results: Vec<_> =
            try_join_all(servers.iter().map(async |(server_name, server_states)| {
                let mut results = route_search_blocks(
                    server_name,
                    server_states,
                    &self.bootstrap.databases,
                    search_method,
                    block_ids.clone(),
                    Some(query.clone()),
                    query_vector.clone(),
                    top_n,
                )
                .await?;

                results.sort_by(|left, right| right.score.total_cmp(&left.score));
                results.truncate(top_n);

                let filter = BlockQuery::ByIds(results.iter().map(|item| item.block_id).collect());

                route_read_blocks(
                    &server_name,
                    &server_states,
                    &self.bootstrap.databases,
                    &filter,
                    false,
                    true,
                )
                .await
            }))
            .await?
            .into_iter()
            .flatten()
            .collect();

        Ok(results)
    }

    async fn read_blocks(&self, request: MCPReadBlocksRequest) -> Result<Vec<Block>> {
        let block_ids = Self::parse_block_ids(request.block_ids)?;

        let filter = match block_ids.is_empty() {
            true => BlockQuery::All,
            false => BlockQuery::ByIds(block_ids),
        };

        let servers = self.server_registry.get_servers_connections();

        let blocks = try_join_all(servers.iter().map(|(server_name, server_states)| {
            route_read_blocks(
                server_name,
                server_states,
                &self.bootstrap.databases,
                &filter,
                false,
                request.has_payload,
            )
        }))
        .await?
        .into_iter()
        .flatten()
        .collect();

        Ok(blocks)
    }
}
