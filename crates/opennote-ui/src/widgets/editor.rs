use gpui::{AppContext, Context, Entity, ParentElement, Render, Styled, div};
use gpui_component::input::{Input, InputState};

pub struct OpenNoteEditor {
    state: Entity<InputState>,
}

impl OpenNoteEditor {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        Self {
            state: cx.new(|cx| {
                InputState::new(window, cx)
                    .code_editor("markdown")
                    .line_number(true)
                    .searchable(false)
            }),
        }
    }
}

impl Render for OpenNoteEditor {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .flex_1() // We need flex_1 to let the editor to take up the whole space after sidebar disappeared
            .child(
                Input::new(&self.state).h_full(), // We need the input to display in full height
            )
    }
}
