use gpui::{
    Entity, IntoElement, ParentElement, RenderOnce,
    Styled, Window, div,
};
use gpui_component::{
    h_flex,
    input::{Input, InputState},
    v_flex,
};

#[derive(IntoElement)]
pub struct SearchBar {
    input_state: Entity<InputState>,
    is_toggled: bool,
}

impl SearchBar {
    pub fn new(input_state: Entity<InputState>, is_toggled: bool) -> Self {
        SearchBar {
            is_toggled,
            input_state,
        }
    }
}

impl RenderOnce for SearchBar {
    fn render(self, _window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        if self.is_toggled {
            return div().absolute().size_full().child(
                v_flex().top_20().items_center().child(
                    h_flex()
                        .w_128() // Apply a default width of the search bar
                        .child(Input::new(&self.input_state)),
                ),
            );
        }

        div()
    }
}
