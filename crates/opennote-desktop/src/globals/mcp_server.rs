use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use futures::future::try_join_all;
use uuid::Uuid;

use opennote_bootstrap::DesktopBootstrap;
use opennote_embedder::vectorization::send_vectorization;
use opennote_mcp_server::{
    requests::{MCPReadBlocksRequest, MCPSearchRequest},
    traits::OpenNoteMCPServiceImplementation,
};
use opennote_models::{
    block::Block, configurations::search::SupportedSearchMethod, payload::create_query,
    query::BlockQuery, search::RawSearchResult,
};

use crate::globals::{
    actions::route_helpers::{route_read_blocks, route_search_blocks},
    server_registry::ServerRegistry,
};

pub struct DesktopMCPServer {
    server_registry: Arc<ServerRegistry>,
    bootstrap: DesktopBootstrap,
}

impl DesktopMCPServer {
    pub fn new(server_registry: Arc<ServerRegistry>, bootstrap: DesktopBootstrap) -> Self {
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
    async fn search(&self, request: MCPSearchRequest) -> Result<Vec<RawSearchResult>> {
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

        let mut results: Vec<_> = try_join_all(self.server_registry.get_servers().iter().map(
            |(server_name, server_states)| {
                route_search_blocks(
                    server_name,
                    server_states,
                    &self.bootstrap.databases,
                    search_method,
                    block_ids.clone(),
                    Some(query.clone()),
                    query_vector.clone(),
                    top_n,
                )
            },
        ))
        .await?
        .into_iter()
        .flatten()
        .collect();

        results.sort_by(|left, right| right.score.total_cmp(&left.score));
        results.truncate(top_n);

        Ok(results)
    }

    async fn read_blocks(&self, request: MCPReadBlocksRequest) -> Result<Vec<Block>> {
        let block_ids = Self::parse_block_ids(request.block_ids)?;

        let filter = match block_ids.is_empty() {
            true => BlockQuery::All,
            false => BlockQuery::ByIds(block_ids),
        };

        let blocks = try_join_all(self.server_registry.get_servers().iter().map(
            |(server_name, server_states)| {
                route_read_blocks(
                    server_name,
                    server_states,
                    &self.bootstrap.databases,
                    &filter,
                    false,
                    true,
                )
            },
        ))
        .await?
        .into_iter()
        .flatten()
        .collect();

        Ok(blocks)
    }
}
