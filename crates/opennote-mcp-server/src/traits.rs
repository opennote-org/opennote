use anyhow::Result;
use async_trait::async_trait;

use crate::requests::{
    MCPCreateBlockRequest, MCPDeleteBlocksRequest, MCPReadBlockDetailsRequest,
    MCPReadBlocksRequest, MCPSearchRequest, MCPUpdateBlocksRequest,
};

#[async_trait]
pub trait OpenNoteMCPServiceImplementation: Send + Sync + 'static {
    /// Allow AI agents to search with both semantic and keyword search across the database
    async fn search(&self, request: MCPSearchRequest) -> Result<String>;

    /// Allow AI agents to get a data brief about the entire database
    async fn read_blocks(&self, request: MCPReadBlocksRequest) -> Result<String>;

    /// Read a block in details, including its full contents
    async fn read_block_details(&self, request: MCPReadBlockDetailsRequest) -> Result<String>;

    /// Allow AI agents to update a block
    async fn update_block(&self, request: MCPUpdateBlocksRequest) -> Result<()>;

    /// Allow AI agents to create a block
    async fn create_block(&self, request: MCPCreateBlockRequest) -> Result<()>;

    /// Allow AI agents to delete blocks
    async fn delete_blocks(&self, request: MCPDeleteBlocksRequest) -> Result<()>;
}
