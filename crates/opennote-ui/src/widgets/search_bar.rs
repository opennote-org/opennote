use gpui::{
    AppContext, Context, FocusHandle, Focusable, InteractiveElement, ParentElement, Render, Styled,
    Window, div, px,
};
use gpui_component::{
    h_flex,
    input::{Input, InputState},
    v_flex,
};

use crate::{library::widget::traits::Widget, views::workspace::Workspace};

pub struct SearchBar {
    focus_handle: FocusHandle,
    previous_focus_handle: Option<FocusHandle>,
    is_toggled: bool,
}

impl SearchBar {
    pub fn new(cx: &mut Context<Workspace>) -> Self {
        let focus_handle = cx.focus_handle();
        dbg!("Sidebar gains focus: ", &focus_handle);
        SearchBar {
            focus_handle,
            previous_focus_handle: None,
            is_toggled: true,
        }
    }
}

// impl EventEmitter<DismissEvent> for SearchBar {}

impl Focusable for SearchBar {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

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
        let input = Input::new(&cx.new(|cx| InputState::new(window, cx)));

        if self.is_toggled {
            return div()
                .track_focus(&self.focus_handle)
                .absolute()
                .size_full()
                .inset_0()
                .occlude()
                .child(
                    v_flex()
                        .h(px(0.0))
                        .top_20()
                        .items_center()
                        .child(h_flex().occlude().child(input)),
                );
        }

        div()
    }

    fn toggle(
        &mut self,
        _action: &dyn gpui::Action,
        window: &mut gpui::Window,
        cx: &mut Context<impl Render>,
    ) {
        self.is_toggled = !self.is_toggled;

        dbg!("Toggle from Sidebar");

        if !self.is_toggled {
            if let Some(previous_focus_handle) = self.previous_focus_handle.take() {
                window.focus(&previous_focus_handle);
                cx.notify();
                return;
            }
        }

        self.previous_focus_handle = window.focused(cx);
        window.focus(&self.focus_handle);
        cx.notify();
    }
}
