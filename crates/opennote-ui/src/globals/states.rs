use std::sync::{Arc, RwLock};

use anyhow::Error;

use gpui::{App, Global};
use opennote_models::block::Block;

#[derive(Debug, Clone)]
pub struct ProtectedBlock(pub Arc<RwLock<Block>>);

impl From<Block> for ProtectedBlock {
    fn from(value: Block) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

/// It manages all business logics related data
pub struct States {
    /*
     * Errors
     */
    pub errors: Arc<RwLock<Vec<Error>>>,

    /*
     * Blocks relevant data
     */
    pub active_block: Option<ProtectedBlock>,
    pub blocks: Arc<RwLock<Vec<ProtectedBlock>>>,
}

impl Global for States {}

impl States {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(Vec::new())),
            active_block: None,
            blocks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn init(cx: &mut App) {
        cx.set_global(States::new());
    }

    /// Overwrite the existing blocks in the states with the new blocks
    pub fn hard_update_blocks(&mut self, blocks: Vec<Block>) {
        let items: Vec<ProtectedBlock> = blocks.into_iter().map(|item| item.into()).collect();
        self.blocks = Arc::new(RwLock::new(items));
    }
}
