use std::collections::HashMap;

use anyhow::Error;

use gpui::{App, AppContext, Global};
use opennote_core_logics::block::read_blocks;
use opennote_data::database::enums::BlockQuery;
use opennote_models::block::Block;
use uuid::Uuid;

use crate::globals::bootstrap::GlobalApplicationBootStrap;

/// It manages all business logics related data
pub struct States {
    /*
     * Errors
     */
    pub errors: Vec<Error>,

    /*
     * Blocks
     */
    pub blocks: HashMap<Uuid, Block>,

    /*
     * Panes
     */
    pub active_pane_id: Option<Uuid>,
}

impl Global for States {}

impl States {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            blocks: HashMap::new(),
            active_pane_id: None,
        }
    }

    pub fn init(cx: &mut App) {
        cx.set_global(States::new());
    }

    /// Overwrite the existing blocks in the states with the new blocks
    pub fn hard_update_blocks(&mut self, blocks: Vec<Block>) {
        self.blocks = HashMap::from_iter(blocks.into_iter().map(|item| (item.id, item)));
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
                        log::error!("{}", error);
                    }
                };
            })
            .detach();
        });
    }
}
