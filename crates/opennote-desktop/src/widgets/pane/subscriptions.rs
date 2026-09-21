use gpui_kit::{Context, Entity};

use opennote_velotype::editor::EditorEvent;
use uuid::Uuid;

use crate::widgets::pane::Pane;

pub fn subscribe_editor_events(
    block_id: Uuid,
    view: &mut Pane,
    _state: &Entity<opennote_velotype::editor::Editor>,
    event: &EditorEvent,
    window: &mut gpui_kit::Window,
    _cx: &mut Context<'_, Pane>,
) {
    match event {
        EditorEvent::ContentChanged => {
            view.opened_tab_states
                .update_tab_save_state(window, &block_id, false);
        }
    }
}
