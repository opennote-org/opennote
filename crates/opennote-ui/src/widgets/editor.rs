use gpui::{AppContext, Context, Entity, ParentElement, Render, Styled, Subscription, div};
use gpui_component::input::{Input, InputState};
use uuid::Uuid;

use opennote_models::block::Block;

// Payload -> Text -> Payload
// Users can edit text then send it back as payloads
// Text is always text in the editor
pub struct Editor {
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

        Input::new(&self.state).h_full() // We need the input to display in full height
    }
}
