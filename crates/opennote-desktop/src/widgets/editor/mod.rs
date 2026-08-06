mod observations;
mod subscriptions;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, ParentElement,
    Render, SharedString, Styled, Subscription, WeakEntity, div,
};
use uuid::Uuid;

use opennote_models::block::Block;

use crate::{
    globals::{
        actions::{block::get_block_content, chunking::chunk_block},
        tasks::tracker::TaskTracker,
    },
    key_mappings::{key_contexts::EDITOR, mappings::SaveDocument},
    widgets::{
        editor::{observations::observe_chunk_block, subscriptions::subscribe_editor_events},
        pane::Pane,
    },
};

/// Payload -> Text -> Payload
/// Users can edit text then send it back as payloads
/// Text is always text in the editor
pub struct Editor {
    focus_handle: FocusHandle,
    pub state: Entity<opennote_velotype::editor::Editor>,

    pub highlighted_text: Option<SharedString>,
    pub block: Option<Block>,
    loaded_block_id: Option<Uuid>,

    /// The pane that owns this editor
    pane: WeakEntity<Pane>,

    _subscriptions: Vec<Subscription>,
}

impl Editor {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window, pane: WeakEntity<Pane>) -> Self {
        let mut _subscriptions = Vec::new();

        let state =
            cx.new(|cx| opennote_velotype::editor::Editor::from_markdown(cx, "".to_string(), None));

        // Get updates from the normal task scheduler
        _subscriptions.push(cx.observe_global_in::<TaskTracker>(window, observe_chunk_block));

        _subscriptions.push(cx.subscribe_in(&state, window, subscribe_editor_events));

        Self {
            focus_handle: cx.focus_handle(),
            state,
            highlighted_text: None,
            block: None,
            loaded_block_id: None,
            pane,
            _subscriptions,
        }
    }

    /// Register a block to the editor for opening
    pub fn register_block(&mut self, cx: &mut App, block: Block) {
        // If the same block has already opened, just return.
        if let Some(existing_block) = &self.block {
            if existing_block.id == block.id {
                return;
            }
        }

        // If the block is unsaved, we will save the unsaved content to the state.
        self.save_unsaved_content_to_tab_state(cx);

        // Swap the block with the new one for opening.
        self.block = Some(block);
    }

    /// Highlight a string in the editor.
    /// It will do nothing if the `highlighted_text` is None.
    pub fn register_highlighted_text(&mut self, highlighted_text: Option<SharedString>) {
        self.highlighted_text = highlighted_text;
    }

    fn apply_highlighted_text(&mut self, cx: &mut Context<'_, Editor>) {
        let text_to_highlight = std::mem::take(&mut self.highlighted_text);
        self.state.update(cx, |this, cx| {
            if let Some(text_to_highlight) = text_to_highlight {
                this.highlight_search_result(cx, text_to_highlight.into());
            }
        });
    }

    fn save_unsaved_content_to_tab_state(&mut self, cx: &mut App) {
        let pane = self.pane.clone();
        let block_id = self.block.as_ref().map(|item| item.id);
        let existing_block_content = self.state.read(cx).get_editor_value(cx).into();

        cx.defer(move |cx| {
            let _ = pane.update(cx, |this, _cx| {
                if let Some(existing_block_id) = &block_id {
                    if let Some(tab_state) = this.opened_block_states.get_mut(&existing_block_id) {
                        tab_state.unsaved_content = Some(existing_block_content);
                    }
                }
            });
        });
    }

    fn has_text_changed(
        block_texts: &str,
        input_state: &Entity<opennote_velotype::editor::Editor>,
        cx: &mut App,
    ) -> bool {
        let current_value = input_state.read(cx).get_editor_value(cx);

        if &current_value == block_texts {
            return false;
        }

        true
    }

    /// Update the editor content with the new openned block's content
    pub fn update_editor_content_with_new_block(&mut self, cx: &mut Context<Self>) {
        let block = match &self.block {
            Some(block) => block,
            None => return,
        };

        // Skip if the block has already opened by this editor
        if let Some(loaded_block_id) = self.loaded_block_id {
            if loaded_block_id == block.id {
                self.apply_highlighted_text(cx);
                return;
            }
        }

        self.loaded_block_id = Some(block.id);

        // If we don't have this block's unsaved content in the state,
        // we will use the block's content directly.
        let unsaved_content = self
            .pane
            .update(cx, |this, _cx| {
                if let Some(tab_state) = this.opened_block_states.get_mut(&block.id) {
                    return tab_state.unsaved_content.take();
                }

                None
            })
            .unwrap();

        let texts = if let Some(unsaved) = unsaved_content {
            unsaved
        } else {
            get_block_content(&block.id, cx).unwrap().into()
        };

        // Early return if the new block is identical with the opened one
        if !Self::has_text_changed(&texts, &self.state, cx) {
            return;
        }

        self.state.update(cx, |this, cx| {
            this.update_editor_content(cx, texts.into());
        });
        self.apply_highlighted_text(cx);
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

/// TODO:
/// - Should we make the Block object a reference?
impl Render for Editor {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.update_editor_content_with_new_block(cx);

        div()
            .key_context(EDITOR)
            .track_focus(&self.focus_handle(cx))
            .h_full()
            .child(
                div().child(self.state.clone()).h_full().border_10(), // We need the input to display in full height
            )
            .on_action(cx.listener(|this, _action: &SaveDocument, window, cx| {
                if let Some(block) = &mut this.block {
                    let text = this.state.read(cx).get_editor_value(cx);
                    // Send the chunking task to the background.
                    // Once finished, editors will pull the results and do the saving.
                    chunk_block(window, cx, block.clone(), text);
                }
            }))
    }
}
