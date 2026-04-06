use gpui::{IntoElement, ParentElement, RenderOnce, Window, div};
use gpui_component::{
    Side, button::Button, h_flex, sidebar::{Sidebar as GPUIComponentSidebar, SidebarMenu, SidebarMenuItem}
};

use crate::globals::helpers::get_language_profile;

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
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
        
        GPUIComponentSidebar::new(Side::Left)
            .child(SidebarMenu::new().child(SidebarMenuItem::new("hello world")))
            .collapsible(true)
            .collapsed(self.is_collapsed)
            .header(
                h_flex()
                    .child(language_profile.sidebar_title)
            )
    }
}
