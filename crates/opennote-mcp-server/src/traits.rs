use anyhow::Result;
use async_trait::async_trait;

use opennote_models::{block::Block, search::RawSearchResult};

use crate::requests::{MCPReadBlocksRequest, MCPSearchRequest};

#[async_trait]
pub trait OpenNoteMCPServiceImplementation: Send + Sync + 'static {
    /// Allow AI agents to search with both semantic and keyword search across the database
    async fn search(&self, request: MCPSearchRequest) -> Result<Vec<RawSearchResult>>;

    /// Allow AI agents to get a data brief about the entire database
    async fn read_blocks(&self, request: MCPReadBlocksRequest) -> Result<Vec<Block>>;
}
