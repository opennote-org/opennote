use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    ParentElement, Render, SharedString, Styled, Subscription, WeakEntity, div,
};
use gpui_component::{
    WindowExt,
    input::{Input, InputEvent, InputState},
};
use uuid::Uuid;

use opennote_models::block::Block;

use crate::{
    globals::{
        actions::{chunk_block, update_n_blocks},
        tasks::{
            task_result::{TaskResult, TaskType},
            tracker::TaskTracker,
            unique_notifications::ChunkBlockNotification,
        },
    },
    key_mappings::{key_contexts::EDITOR, mappings::SaveDocument},
    widgets::pane::{pane::Pane, tab::TabState},
};

/// Payload -> Text -> Payload
/// Users can edit text then send it back as payloads
/// Text is always text in the editor
pub struct Editor {
    focus_handle: FocusHandle,
    state: Entity<InputState>,
    pub block: Option<Block>,
    loaded_block_id: Option<Uuid>,

    /// The pane that owns this editor
    pane: WeakEntity<Pane>,

    _subscriptions: Vec<Subscription>,
}

impl Editor {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window, pane: WeakEntity<Pane>) -> Self {
        let mut _subscriptions = Vec::new();

        // Get updates from the normal task scheduler
        let pane_clone: WeakEntity<Pane> = pane.clone();
        _subscriptions.push(cx.observe_global_in::<TaskTracker>(
            window,
            move |this, window, cx| {
                let Some(block) = &this.block else {
                    return;
                };

                let scheduler: &TaskTracker = cx.global();
                if !scheduler.has_pending_task_results(Some(TaskType::ChunkBlock(block.id))) {
                    return;
                }

                let task_result =
                    cx.update_global::<TaskTracker, Option<TaskResult>>(|this, _cx| {
                        this.get_task_result(TaskType::ChunkBlock(block.id))
                    });

                if let Some(result) = task_result {
                    window.remove_notification::<ChunkBlockNotification>(cx);

                    let block: Block = if let Some(data) = result.data {
                        serde_json::from_value(data).unwrap()
                    } else {
                        return;
                    };

                    update_n_blocks(window, cx, vec![block.clone()], true);
                    cx.notify();
                }

                // Alter the tab's save state to true
                TabState::set_save_state(cx, pane_clone.clone(), block.id, true);
            },
        ));

        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("markdown")
                .line_number(true)
                .searchable(true) // It will search with the backend instead
        });

        let pane_clone: WeakEntity<Pane> = pane.clone();
        _subscriptions.push(cx.subscribe_in(
            &state,
            window,
            move |view, state, event, _window, cx| match event {
                InputEvent::Change => {
                    let Some(block) = &view.block else {
                        return;
                    };

                    let texts: String = block.get_text_content();

                    if !Self::has_text_changed(&texts, state, cx) {
                        return;
                    }

                    TabState::set_save_state(cx, pane_clone.clone(), block.id, false);
                }
                _ => {}
            },
        ));

        Self {
            focus_handle: cx.focus_handle(),
            state,
            block: None,
            loaded_block_id: None,
            pane,
            _subscriptions,
        }
    }

    pub fn register_block(
        &mut self,
        cx: &mut App,
        window: &mut gpui::Window,
        block: Block,
        highlighted_text: Option<SharedString>,
    ) {
        // If the same block is already open, just update the highlight and return.
        if let Some(existing_block) = &self.block {
            if existing_block.id == block.id {
                self.set_highlighted_text(cx, window, highlighted_text);
                return;
            }
        }

        // If the block is unsaved, we will save the unsaved content to the state.
        self.save_unsaved_content_to_tab_state(cx);

        // Swap the block with the new one for opening.
        self.block = Some(block);

        self.set_highlighted_text(cx, window, highlighted_text);
    }

    /// Highlight a string in the editor.
    /// It will do nothing if the `highlighted_text` is None.
    fn set_highlighted_text(
        &mut self,
        cx: &mut App,
        window: &mut gpui::Window,
        highlighted_text: Option<SharedString>,
    ) {
        let Some(string) = highlighted_text else {
            return;
        };

        self.state.update(cx, |this, cx| {
            this.set_highlighted_text(cx, window, string);
        });
    }

    fn save_unsaved_content_to_tab_state(&mut self, cx: &mut App) {
        let pane = self.pane.clone();
        let block_id = self.block.as_ref().map(|item| item.id);
        let existing_block_content = self.state.read(cx).value();

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

    fn has_text_changed(block_texts: &str, input_state: &Entity<InputState>, cx: &mut App) -> bool {
        let current_value = input_state.read(cx).value();

        if current_value.as_ref() == block_texts {
            return false;
        }

        true
    }

    /// Update the editor content with the new openned block's content
    pub fn update_editor_content_with_new_block(
        &mut self,
        cx: &mut Context<Self>,
        window: &mut gpui::Window,
    ) {
        let block = match &self.block {
            Some(block) => block,
            None => return,
        };

        // Skip if the block has already opened by this editor
        if let Some(loaded_block_id) = self.loaded_block_id {
            if loaded_block_id == block.id {
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
            block.get_text_content().into()
        };

        // Early return if the new block is identical with the opened one
        if !Self::has_text_changed(&texts, &self.state, cx) {
            return;
        }

        self.state
            .update(cx, |this, cx| this.set_value(texts, window, cx));
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
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.update_editor_content_with_new_block(cx, window);

        div()
            .key_context(EDITOR)
            .track_focus(&self.focus_handle(cx))
            .h_full()
            .child(
                Input::new(&self.state).h_full().bordered(false), // We need the input to display in full height
            )
            .on_action(cx.listener(|this, _action: &SaveDocument, window, cx| {
                if let Some(block) = &mut this.block {
                    let text = this.state.read(cx).value().to_string();
                    // Send the chunking task to the background.
                    // Once finished, editors will pull the results and do the saving.
                    chunk_block(window, cx, block.clone(), text);
                }
            }))
    }
}
