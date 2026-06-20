use gpui::{App, SharedString};
use uuid::Uuid;

use crate::globals::states::States;

/// Open a block to the active pane
pub fn open_block(cx: &mut App, block_id: Uuid, highlighted_text: Option<SharedString>) {
    let states: &States = cx.global();
    let Some(active_pane) = states.active_pane.clone() else {
        return;
    };

    // Set the selected block in the active pane
    let _ = active_pane.update(cx, |this, cx| {
        this.set_selected_block_by_block_id(block_id, cx);
        log::debug!("Opened block: {}", block_id);

        if let Some(string) = highlighted_text {
            this.set_search_string(string.clone());
            log::debug!("Highlighted block: {} at {}", block_id, &string);
        }
    });
}
