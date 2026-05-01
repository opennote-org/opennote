use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use opennote_models::block::Block;

use crate::database::enums::BlockQuery;

#[async_trait]
pub trait Blocks {
    /// Create blocks in the database. 
    /// It will also create their belonging payloads. 
    async fn create_blocks(&self, blocks: Vec<Block>) -> Result<Vec<Block>>;

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
