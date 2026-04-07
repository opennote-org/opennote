//! States manage all business logics data

use std::sync::{Arc, RwLock};

use anyhow::Error;

use gpui::{App, Global};
use opennote_models::block::Block;

pub struct States {
    pub errors: Arc<RwLock<Vec<Error>>>,
    pub blocks: Arc<RwLock<Vec<Block>>>,
}

impl Global for States {}

impl States {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(Vec::new())),
            blocks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn init(cx: &mut App) {
        cx.set_global(States::new());
    }
}
