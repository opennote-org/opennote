use serde::{Deserialize, Serialize};

use crate::block::Block;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestReindexBlocksResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendReindexedBlocksResponse {
    /// Blocks to reindex
    pub blocks: Vec<Block>,
    /// All blocks finished sending, after this request
    pub finished: bool,
}
