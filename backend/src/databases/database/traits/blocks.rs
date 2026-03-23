use anyhow::Result;
use async_trait::async_trait;

use crate::models::block::Block;

#[async_trait]
pub trait Blocks {
    /// Create 
    async fn create_blocks() -> Result<Vec<Block>>;
    
    /// This is to get the blocks without a parent
    async fn read_blocks(filter: ReadBlockFilter) -> Result<Vec<Block>>;
    
    /// Update a block by passing the blocks to update 
    async fn update_blocks(blocks: Vec<Block>) -> Result<Vec<Block>>;
    
    /// Delete blocks by their ids
    /// Children blocks will be removed as well
    async fn delete_blocks(block_ids: Vec<String>) -> Result<Vec<Block>>;
}

/// Leve all empty to get root blocks
/// One filter at a time
pub struct ReadBlockFilter {
    /// Get blocks by specifying their ids
    pub ids: Vec<String>,
    /// Speicify parent ids to get their children blocks
    pub parent_ids: Vec<String>,
}