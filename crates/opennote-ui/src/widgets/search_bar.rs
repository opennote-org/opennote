use gpui::{
    AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, Render, Styled, Window, div, prelude::FluentBuilder, rems,
};
use gpui_component::{
    StyledExt,
    input::{Input, InputState},
    menu::PopupMenu,
    v_flex,
};

use crate::library::widget::traits::Widget;

pub struct SearchBar {
    // focus_handler: FocusHandle,
    is_toggled: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        SearchBar {
            // focus_handler: cx.focus_handle(),
            is_toggled: true,
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
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.create(window, cx)
    }
}

impl Widget for SearchBar {
    fn create(&self, window: &mut Window, cx: &mut Context<impl Render>) -> impl gpui::IntoElement {
        dbg!("created searchbar");
        Input::new(&cx.new(|cx| InputState::new(window, cx))).debug_blue()
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
