use std::collections::HashMap;

use uuid::Uuid;

use crate::libs::tree::TreeItem;
use opennote_models::block::Block;

/// Build a `TreeItem` hierarchy from blocks, ready to pass into `TreeState::items()`.
pub fn build_blocks_tree(blocks: Vec<Block>) -> Vec<TreeItem> {
    let mut map: HashMap<Option<Uuid>, Vec<Block>> = HashMap::new();

    // We need the root blocks for starting the recursion
    for block in blocks {
        map.entry(block.parent_id).or_default().push(block);
    }

    let mut tree_items = vec![TreeItem::new(Uuid::new_v4().to_string(), "root")]; // Reserved for being able to drag blocks back to root
    tree_items.extend(build_children(None, &mut map));

    tree_items
}

fn build_children(
    parent_id: Option<Uuid>,
    map: &mut HashMap<Option<Uuid>, Vec<Block>>,
) -> Vec<TreeItem> {
    let Some(siblings) = map.get(&parent_id) else {
        return Vec::new();
    };

    let siblings = siblings.clone();

    siblings
        .iter()
        .map(|block| {
            let (id, label) = { (block.id, block.get_title()) };

            let children = build_children(Some(id), map);

            // Use the UUID string as the TreeItem id so we can recover the block on selection
            TreeItem::new(id.to_string(), label)
                .expanded(true)
                .children(children)
        })
        .collect()
}
