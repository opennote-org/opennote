use gpui::{App, SharedString};
use uuid::Uuid;

use crate::globals::states::States;

/// TODO:
/// - click to jump to the corresponding block and payload

/// Open a block to the active pane
pub fn open_block(cx: &mut App, block_id: Uuid) {
    let states: &States = cx.global();
    let Some(active_pane) = states.active_pane.clone() else {
        return;
    };

    // Set the selected block in the active pane
    let _ = active_pane.update(cx, |this, cx| {
        this.set_selected_block_by_block_id(block_id, cx);
    });
}
