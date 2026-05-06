use gpui::{
    AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, ParentElement, Render,
    Styled, Subscription, div,
};
use gpui_component::input::{Input, InputState};

use opennote_core_logics::payload::convert_string_to_payloads;
use opennote_models::block::Block;

use crate::{
    globals::{actions::update_n_blocks, bootstrap::GlobalApplicationBootStrap, states::States},
    key_mappings::{key_contexts::EDITOR, mappings::SaveDocument},
};

// Payload -> Text -> Payload
// Users can edit text then send it back as payloads
// Text is always text in the editor
pub struct Editor {
    focus_handle: FocusHandle,
    state: Entity<InputState>,
    block: Option<Block>,

    /// Whether this payload has been loaded into texts already
    is_text_preloaded: bool,

    _subscriptions: Vec<Subscription>,
}

impl Editor {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        let mut _subscriptions = Vec::new();

        Self {
            focus_handle: cx.focus_handle(),
            state: cx.new(|cx| {
                InputState::new(window, cx)
                    .code_editor("markdown")
                    .line_number(true)
                    .searchable(false)
            }),
            block: None,
            is_text_preloaded: false,
            _subscriptions,
        }
    }

    pub fn register_block(&mut self, block: Block) {
        self.block = Some(block);
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Editor {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        // We only load the payload for once.
        // Editing is exerted directly on texts, not payloads.
        if !self.is_text_preloaded {
            if let Some(block) = &self.block {
                let texts: Vec<String> = block
                    .payloads
                    .iter()
                    .map(|item| item.texts.clone())
                    .collect();
                let texts: String = texts.concat();

                let current_value = self.state.read(cx).value();
                if current_value.as_ref() != texts.as_str() {
                    self.state
                        .update(cx, |this, cx| this.set_value(texts, window, cx));
                }

                self.is_text_preloaded = true;
            }
        }

        div()
            .key_context(EDITOR)
            .track_focus(&self.focus_handle(cx))
            .h_full()
            .child(
                Input::new(&self.state).h_full(), // We need the input to display in full height
            )
            .on_action(cx.listener(|this, _action: &SaveDocument, _window, cx| {
                // 1. slice the string into payloads
                if let Some(block) = &mut this.block {
                    let input_state = this.state.read(cx);
                    let bootstrap: &GlobalApplicationBootStrap = cx.global();

                    let payloads = match convert_string_to_payloads(
                        block.id,
                        Some(bootstrap.0.configurations.system.embedder.dimensions),
                        input_state.value().to_string(),
                    ) {
                        Ok(results) => results,
                        Err(error) => {
                            log::error!("Error when trying to save a document: {}", error);
                            return;
                        }
                    };

                    // 2. swap the payloads into the block
                    block.payloads = payloads;

                    // 3. update blocks
                    update_n_blocks(cx, vec![block.clone()]);
                    cx.notify();
                }
            }))
    }
}
