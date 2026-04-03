use gpui::{Action, Context, FocusHandle, Focusable, InteractiveElement, ParentElement, Render, Window, div};
use gpui_component::{
    Side,
    sidebar::{Sidebar as GPUIComponentSidebar, SidebarMenu, SidebarMenuItem},
};

use crate::{library::widget::traits::Widget, views::workspace::Workspace};

pub struct Sidebar {
    focus_handle: FocusHandle,
    is_collapsed: bool,
}

impl Sidebar {
    pub fn new(cx: &mut Context<Workspace>) -> Self {
        let focus_handle = cx.focus_handle();
        dbg!("Sidebar gains focus: ", &focus_handle);
        Self {
            focus_handle,
            is_collapsed: false,
        }
    }
}

impl Focusable for Sidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Widget for Sidebar {
    fn create(
        &self,
        _window: &mut Window,
        _cx: &mut Context<impl Render>,
    ) -> impl gpui::IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .child(
                GPUIComponentSidebar::new(Side::Left)
                    .child(SidebarMenu::new().child(SidebarMenuItem::new("hello world")))
                    .collapsible(true)
                    .collapsed(self.is_collapsed)
            )
    }

    fn toggle(
        &mut self,
        _action: &dyn Action,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<impl Render>,
    ) {
        dbg!("Toggle from Sidebar");
        self.is_collapsed = !self.is_collapsed;
        cx.notify();
    }
}
