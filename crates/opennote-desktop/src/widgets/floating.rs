use gpui::{Div, FocusHandle, InteractiveElement, Styled, div, prelude::FluentBuilder};

pub fn create_float_palette(focus_handle: &FocusHandle, is_toggled: bool) -> Div {
    div()
        .track_focus(focus_handle)
        .absolute()
        .size_full()
        .flex()
        .justify_center()
        .items_center()
        .when(is_toggled, |this| this.visible().occlude())
        .when(!is_toggled, |this| this.invisible())
}
