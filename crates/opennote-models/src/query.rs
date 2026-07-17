use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum PayloadQuery {
    All,                   // All blocks in the database
    ByIds(Vec<Uuid>),      // Specific payloads
    ByBlockIds(Vec<Uuid>), // By payloads' block ids
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum BlockQuery {
    All,                   // All blocks in the database
    Root,                  // Blocks without parent
    ByIds(Vec<Uuid>),      // Specific blocks
    ChildrenOf(Vec<Uuid>), // By parent ids
}
