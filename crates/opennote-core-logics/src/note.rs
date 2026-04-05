//! this file defines logics regarding notes managements

use anyhow::Result;

use opennote_data::{Databases, database::enums::BlockQuery};
use opennote_models::block::Block;

/// Create num of empty notes
/// You can update them with the update function
pub async fn create_blocks(databases: &Databases, num_blocks: usize) -> Result<Vec<Block>> {
    databases.create_blocks(num_blocks).await
}

/// Read blocks from the database
pub async fn read_blocks(databases: &Databases, filter: &BlockQuery) -> Result<Vec<Block>> {
    databases.read_blocks(filter).await
}
