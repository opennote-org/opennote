use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use opennote_models::{constants::DEFAULT_BLOCK_STATES_FILE_NAME, traits::LoadFromAndSaveToFile};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockState {
    pub has_expanded: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BlockStates(HashMap<Uuid, BlockState>);

impl BlockStates {
    pub fn toggle_expansion(&mut self, uuid: Uuid) {
        let block_state = self
            .0
            .entry(uuid)
            .or_insert(BlockState { has_expanded: true });

        block_state.has_expanded = !block_state.has_expanded;
    }

    pub fn get_block_state(&self, uuid: Uuid) -> Option<&BlockState> {
        self.0.get(&uuid)
    }

    /// Prune the blocks that are no longer existed away from the block states.
    pub fn prune(&mut self, existing_block_ids: &[Uuid]) {
        debug!("Number of block states before pruning: {}", self.0.len());

        self.0.retain(|item, _| existing_block_ids.contains(item));

        debug!("Number of block states after pruning: {}", self.0.len());
    }
}

impl LoadFromAndSaveToFile for BlockStates {
    fn get_configuration_filename() -> &'static str {
        DEFAULT_BLOCK_STATES_FILE_NAME
    }
}
