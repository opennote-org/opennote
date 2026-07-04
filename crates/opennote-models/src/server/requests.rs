use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{block::Block, configurations::search::SupportedSearchMethod};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlocksInWorkspaceRequest {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlocksInWorkspaceRequest {
    pub block_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBlocksInWorkspaceRequest {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBlocksInWorkspaceRequest {
    pub search_method: SupportedSearchMethod,
    pub block_ids: Vec<Uuid>,
    pub query: Option<String>,
    pub query_vector: Option<Vec<f32>>,
    pub top_n: usize,
}
