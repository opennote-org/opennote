use gpui::{Context, Focusable, InteractiveElement, ParentElement, Styled, div};

use crate::{
    globals::{actions::chunking::chunk_block, states::helpers::get_states},
    key_mappings::{key_contexts::EDITOR, mappings::SaveDocument},
    widgets::pane::Pane,
};

impl Pane {
    pub fn render_editor(&self, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        if let Some(editor) = &self.editor {
            let editor_clone = editor.clone();
            return div()
                .key_context(EDITOR)
                .track_focus(&self.focus_handle(cx))
                .h_full()
                .child(
                    div().child(editor.clone()).h_full().border_10(), // We need the input to display in full height
                )
                .on_action(
                    cx.listener(move |this, _action: &SaveDocument, window, cx| {
                        if let Some(block_id) = &mut this.selected_block_id {
                            let text = editor_clone.read(cx).get_editor_value(cx);
                            let states = get_states(cx);
                            if let Some(block) = states.get_block(block_id) {
                                // Send the chunking task to the background.
                                // Once finished, editors will pull the results and do the saving.
                                chunk_block(window, cx, block.clone(), text);
                            }
                        }
                    }),
                );
        }

        div()
    }
}
