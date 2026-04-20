use gpui::{Entity, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder};
use gpui_component::{
    h_flex,
    input::{Input, InputState},
    v_flex,
};

pub fn create_search_bar(input_state: &Entity<InputState>, is_toggled: bool) -> impl IntoElement {
    div()
        .absolute()
        .size_full()
        .when(is_toggled, |this| this.visible())
        .when(!is_toggled, |this| this.invisible())
        .child(
            v_flex().top_20().items_center().child(
                h_flex()
                    .w_128() // Apply a default width of the search bar
                    .child(Input::new(input_state)),
            ),
        )
}
