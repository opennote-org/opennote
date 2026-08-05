use gpui::{Context, Entity};

use opennote_velotype::editor::EditorEvent;

use crate::widgets::{editor::Editor, pane::tab::TabState};

pub fn subscribe_editor_events(
    view: &mut Editor,
    _state: &Entity<opennote_velotype::editor::Editor>,
    event: &EditorEvent,
    _window: &mut gpui::Window,
    cx: &mut Context<'_, Editor>,
) {
    let pane_clone = view.pane.clone();

    match event {
        EditorEvent::ContentChanged => {
            let Some(block) = &view.block else {
                return;
            };

            TabState::set_save_state(cx, pane_clone.clone(), block.id, false);
        }
    }
}
