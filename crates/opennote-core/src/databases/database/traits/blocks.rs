use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::{databases::database::enums::BlockQuery, models::block::Block};

#[async_trait]
pub trait Blocks {
    /// Create num_blocks of empty blocks
    async fn create_blocks(&self, num_blocks: usize) -> Result<Vec<Block>>;

    /// Get blocks with a query filter
    async fn read_blocks(&self, filter: &BlockQuery) -> Result<Vec<Block>>;

    /// Update blocks by passing the blocks to update
    async fn update_blocks(&self, blocks: Vec<Block>) -> Result<()>;

    /// Delete blocks by their ids
    /// Children blocks will be removed as well
    async fn delete_blocks(&self, block_ids: Vec<Uuid>) -> Result<()>;

    /// Get paths to the block
    ///
    /// In the returned list, the first block is the root, and the last block is
    /// the block_id passed in
    async fn read_block_path(&self, block_id: Uuid) -> Result<Vec<Block>>;
}
