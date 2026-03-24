use anyhow::Result;
use async_trait::async_trait;

use crate::models::block::Block;

#[async_trait]
pub trait Blocks {
    /// Create num_blocks of empty blocks
    async fn create_blocks(&self, num_blocks: usize) -> Result<Vec<Block>>;

    /// This is to get the blocks without a parent
    async fn read_blocks(filter: &BlockQuery) -> Result<Vec<Block>>;

    /// Update a block by passing the blocks to update
    async fn update_blocks(blocks: Vec<Block>) -> Result<Vec<Block>>;

    /// Delete blocks by their ids
    /// Children blocks will be removed as well
    async fn delete_blocks(block_ids: Vec<String>) -> Result<Vec<Block>>;
}

pub enum BlockQuery {
    Root,                    // blocks without parent
    ByIds(Vec<String>),      // specific blocks
    ChildrenOf(Vec<String>), // by parent ids
}

impl From<crate::entity::blocks::Model> for Block {
    fn from(value: crate::entity::blocks::Model) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent_id,
            is_deleted: value.is_deleted,
            payloads: vec![],
        }
    }
}
