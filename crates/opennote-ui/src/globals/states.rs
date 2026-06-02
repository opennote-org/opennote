use std::collections::HashMap;

use gpui::{App, AppContext, Global, WeakEntity};
use opennote_core_logics::block::read_blocks;
use opennote_data::database::enums::BlockQuery;
use opennote_models::block::Block;
use uuid::Uuid;

use crate::{globals::bootstrap::GlobalApplicationBootStrap, widgets::pane::pane::Pane};

/// It manages general global states
pub struct States {
    /// Blocks in hash map
    pub blocks: HashMap<Uuid, Block>,

    /// The pane that is active.
    /// It is optional because we can't create a pane when new.
    pub active_pane: Option<WeakEntity<Pane>>,
}

impl Global for States {}

impl States {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            active_pane: None,
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
