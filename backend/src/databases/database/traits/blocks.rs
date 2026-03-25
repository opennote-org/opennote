use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::models::block::Block;

#[async_trait]
pub trait Blocks {
    /// Create num_blocks of empty blocks
    async fn create_blocks(&self, num_blocks: usize) -> Result<Vec<Block>>;

    /// This is to get the blocks without a parent
    async fn read_blocks(&self, filter: &BlockQuery) -> Result<Vec<Block>>;

    /// Update a block by passing the blocks to update
    async fn update_blocks(&self, blocks: Vec<Block>) -> Result<()>;

    /// Delete blocks by their ids
    /// Children blocks will be removed as well
    async fn delete_blocks(&self, block_ids: Vec<Uuid>) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum BlockQuery {
    Root,                    // blocks without parent
    ByIds(Vec<String>),      // specific blocks
    ChildrenOf(Vec<String>), // by parent ids
}
