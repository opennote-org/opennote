use gpui::{Action, Context, Render, Window};
use gpui_component::{
    Side,
    sidebar::{Sidebar as GPUIComponentSidebar, SidebarMenu, SidebarMenuItem},
};

use crate::library::widget::traits::Widget;

pub struct Sidebar {
    is_collapsed: bool,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            is_collapsed: false,
        }
    }
}

impl Widget for Sidebar {
    fn initialize() -> Self {
        Self {
            is_collapsed: false,
        }
    }

    fn create(
        &self,
        _window: &mut Window,
        _cx: &mut Context<impl Render>,
    ) -> impl gpui::IntoElement {
        GPUIComponentSidebar::new(Side::Left)
            .child(SidebarMenu::new().child(SidebarMenuItem::new("hello world")))
            .collapsible(true)
            .collapsed(self.is_collapsed)
    }

    fn toggle(
        &mut self,
        _action: &dyn Action,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<impl Render>,
    ) {
        self.is_collapsed = !self.is_collapsed;
        cx.notify();
    }
}
