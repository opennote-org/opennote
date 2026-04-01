use gpui::{
    Context, DismissEvent, EventEmitter, FocusHandle, Focusable, InteractiveElement, Render,
    Styled, div, rems,
};
use gpui_component::StyledExt;

use crate::library::widget::traits::Widget;

pub struct SearchBar {
    // focus_handler: FocusHandle,
    is_toggled: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        SearchBar {
            // focus_handler: cx.focus_handle(),
            is_toggled: false,
        }
    }
}

// impl EventEmitter<DismissEvent> for SearchBar {}

// impl Focusable for SearchBar {
//     fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
//         self.focus_handler.clone()
//     }
// }

impl Render for SearchBar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.create()
    }
}

impl Widget for SearchBar {
    fn create(&self) -> impl gpui::IntoElement {
        div()
            .v_flex()
            .id("workspace_search_bar")
            .key_context("workspace_search_bar")
            .w(rems(34.0))
    }

    fn initialize() -> Self {
        Self::new()
    }

    fn toggle(
        &mut self,
        _action: &dyn gpui::Action,
        _window: &mut gpui::Window,
        cx: &mut Context<impl Render>,
    ) {
        self.is_toggled = !self.is_toggled;
        cx.notify();
    }
}
