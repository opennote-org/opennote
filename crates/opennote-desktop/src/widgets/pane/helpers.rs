use gpui::{App, SharedString};
use uuid::Uuid;

use crate::globals::states::helpers::get_states;

/// Open a block to the active pane
pub fn open_block(cx: &mut App, block_id: Uuid, highlighted_text: Option<SharedString>) {
    let states = get_states(cx);
    let Some(active_pane) = states.get_active_pane(cx) else {
        return;
    };

    // Set the selected block in the active pane
    let _ = active_pane.update(cx, |this, cx| {
        this.set_selected_block_by_block_id(block_id, cx);

        // Request the editor to focus itself after opening the block.
        this.editor.update(cx, |this, cx| {
            this.request_editor_to_focus(cx);
        });

        if let Some(string) = highlighted_text {
            this.set_search_string(string.clone());
            cx.notify();
        }
    });
}
