use std::sync::{Arc, RwLock};

use anyhow::Error;

use gpui::{App, AppContext, Global};
use opennote_core_logics::block::read_blocks;
use opennote_data::database::enums::BlockQuery;
use opennote_models::block::Block;

use crate::globals::bootstrap::GlobalApplicationBootStrap;

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

    /// Set the block for user operations
    pub fn set_active_block(&mut self, block: ProtectedBlock) {
        self.active_block = Some(block)
    }

    /// Overwrite the existing blocks in the states with the new blocks
    pub fn hard_update_blocks(&mut self, blocks: Vec<Block>) {
        let items: Vec<ProtectedBlock> = blocks.into_iter().map(|item| item.into()).collect();
        self.blocks = Arc::new(RwLock::new(items));
    }
     
    pub fn refresh_blocks_list(cx: &mut App) {
        log::debug!("Refreshing blocks...");

        cx.read_global::<GlobalApplicationBootStrap, ()>(|this, _app| {
            let databases = this.0.databases.clone();

            cx.spawn(async move |cx| {
                match read_blocks(&databases, &BlockQuery::All).await {
                    Ok(results) => {
                        match cx.update_global::<States, ()>(|this, _cx| {
                            this.hard_update_blocks(results);
                        }) {
                            Ok(_) => {}
                            Err(error) => log::error!("{}", error),
                        }
                    }
                    Err(error) => {
                        // cx.read_global::<States, ()>(|this, _cx| {
                        //     this.errors.write().unwrap().push(error);
                        // })
                        // .unwrap();
                        log::error!("{}", error);
                    }
                };
            })
            .detach();
        });
    }
}
