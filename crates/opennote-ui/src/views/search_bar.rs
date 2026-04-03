use gpui::{
    AppContext, FocusHandle, Focusable, InteractiveElement, ParentElement, Render, Styled, div, px,
};
use gpui_component::{
    h_flex,
    input::{Input, InputState},
    v_flex,
};

pub struct SearchBar {
    is_toggled: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        SearchBar { is_toggled: false }
    }

    pub fn toggle(&mut self) {
        self.is_toggled = !self.is_toggled;
    }
}

// impl EventEmitter<DismissEvent> for SearchBar {}

impl Focusable for SearchBar {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        cx.focus_handle()
    }
}

impl Render for SearchBar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let input = Input::new(&cx.new(|cx| InputState::new(window, cx)));

        if self.is_toggled {
            return div().absolute().size_full().inset_0().occlude().child(
                v_flex()
                    .h(px(0.0))
                    .top_20()
                    .items_center()
                    .child(h_flex().occlude().child(input)),
            );
        }

        div()
    }
}

// impl Widget for SearchBar {
//     fn create(&self, window: &mut Window, cx: &mut Context<impl Render>) -> impl gpui::IntoElement {
//     }

//     fn initialize() -> Self {
//         Self::new()
//     }

//     fn toggle(
//         &mut self,
//         _action: &dyn gpui::Action,
//         _window: &mut gpui::Window,
//         cx: &mut Context<impl Render>,
//     ) {
//         self.is_toggled = !self.is_toggled;
//         cx.notify();
//     }
// }
