use gpui::{IntoElement, RenderOnce, Window};
use gpui_component::{
    Side,
    sidebar::{Sidebar as GPUIComponentSidebar, SidebarMenu, SidebarMenuItem},
};

#[derive(IntoElement)]
pub struct Sidebar {
    is_collapsed: bool,
}

impl Sidebar {
    pub fn new(is_collapsed: bool) -> Self {
        Self {
            is_collapsed,
        }
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        GPUIComponentSidebar::new(Side::Left)
            .child(SidebarMenu::new().child(SidebarMenuItem::new("hello world")))
            .collapsible(true)
            .collapsed(self.is_collapsed)
    }
}
