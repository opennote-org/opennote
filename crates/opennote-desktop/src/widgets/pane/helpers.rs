use gpui::{App, SharedString, Window};
use uuid::Uuid;

use crate::globals::states::helpers::get_states;

/// Open a block to the active pane
pub fn open_block(
    cx: &mut App,
    window: &mut Window,
    block_id: Uuid,
    highlighted_text: Option<SharedString>,
) {
    let states = get_states(cx);
    let Some(active_pane) = states.get_active_pane(cx) else {
        return;
    };

    // Set the selected block in the active pane
    let _ = active_pane.update(cx, |this, cx| {
        this.open_or_activate_tab(block_id, cx, window);

        if let Some(string) = highlighted_text {
            this.set_search_string(string.clone());
            cx.notify();
        }
    });
}
