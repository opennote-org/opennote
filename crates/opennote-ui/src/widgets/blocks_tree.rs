use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use uuid::Uuid;

use crate::{globals::states::ProtectedBlock, libs::tree::TreeItem};

/// Build a `TreeItem` hierarchy from blocks, ready to pass into `TreeState::items()`.
pub fn build_blocks_tree(blocks: Arc<RwLock<Vec<ProtectedBlock>>>) -> Vec<TreeItem> {
    let blocks = blocks.read().unwrap().clone();
    let mut map: HashMap<Option<Uuid>, Vec<ProtectedBlock>> = HashMap::new();

    for block in blocks {
        let parent_id = block.0.read().unwrap().parent_id;
        map.entry(parent_id).or_default().push(block);
    }

    build_children(None, &mut map)
}

fn build_children(
    parent_id: Option<Uuid>,
    map: &mut HashMap<Option<Uuid>, Vec<ProtectedBlock>>,
) -> Vec<TreeItem> {
    let Some(siblings) = map.get(&parent_id) else {
        return Vec::new();
    };

    let siblings = siblings.clone();

    siblings
        .iter()
        .map(|block| {
            let (id, label) = {
                let block = block.0.read().unwrap();
                (block.id, block.get_title())
            };

            let children = build_children(Some(id), map);

            // Use the UUID string as the TreeItem id so we can recover the block on selection
            TreeItem::new(id.to_string(), label)
                .expanded(true)
                .children(children)
        })
        .collect()
}
