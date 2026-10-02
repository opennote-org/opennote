use std::collections::HashMap;

use uuid::Uuid;

use opennote_models::block::Block;

#[derive(Debug)]
pub struct ReindexSessionManager(Option<ReindexSession>);

impl ReindexSessionManager {
    pub fn new() -> Self {
        Self(None)
    }

    /// Only one reindex session is allowed at a time.
    /// The existing session will be popped if a new one comes.
    pub fn new_session(&mut self, blocks: Vec<Block>) {
        let session = ReindexSession::new(blocks);
        self.0 = Some(session);
    }

    pub fn get_session(&self) -> Option<&ReindexSession> {
        self.0.as_ref()
    }

    pub fn get_session_mut(&mut self) -> Option<&mut ReindexSession> {
        self.0.as_mut()
    }

    /// End an ongoing session
    pub fn end_session(&mut self) {
        self.0 = None
    }
}

#[derive(Debug)]
pub struct ReindexSession {
    finished: HashMap<Uuid, Block>,
    unfinished: Vec<Block>,
}

impl ReindexSession {
    pub fn new(blocks_to_reindex: Vec<Block>) -> Self {
        Self {
            finished: HashMap::new(),
            unfinished: blocks_to_reindex,
        }
    }

    /// This method will handle deduplications
    pub fn add_blocks(&mut self, blocks: Vec<Block>) {
        if blocks.is_empty() {
            return;
        }

        for block in blocks {
            self.finished.insert(block.id, block);
        }
    }

    /// Take N unfinished blocks
    pub fn take_unfinished_blocks(&mut self, number_blocks: usize) -> Vec<Block> {
        let count = number_blocks.min(self.unfinished.len());
        self.unfinished.drain(..count).collect()
    }

    pub fn has_session_finished(&self) -> bool {
        self.unfinished.is_empty()
    }

    /// Get all finished blocks
    pub fn get_session_contents(&self) -> Vec<Block> {
        self.finished
            .values() // Get values only
            .cloned() // Clone each element and create a new iterator
            .collect()
    }
}
