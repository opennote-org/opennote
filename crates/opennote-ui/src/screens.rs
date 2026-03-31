use gpui::*;
use gpui_component::{
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    *,
};

use crate::globals::UIApplicationBootStrap;

actions!(workspace_sidebar, [ToggleWorkspaceSidebar]);

pub struct MainWindow {
    is_workspace_sidebar_collapsed: bool,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            is_workspace_sidebar_collapsed: false,
        }
    }

    pub fn toggle_workspace_sidebar(&mut self, cx: &mut Context<Self>) {
        self.is_workspace_sidebar_collapsed = !self.is_workspace_sidebar_collapsed;
        cx.notify();
    }

    pub fn render_sidebar(&self) -> Sidebar<SidebarMenu> {
        Sidebar::new(Side::Left)
            .child(SidebarMenu::new().child(SidebarMenuItem::new("hello world")))
            .collapsible(true)
            .collapsed(self.is_workspace_sidebar_collapsed)
    }
}

impl Render for MainWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // TODO: we are able to access the global states from here
        let services_and_resources: &UIApplicationBootStrap = cx.global();

        div()
            .v_flex()
            .id("workspace-sidebar")
            .h_full()
            .child(self.render_sidebar())
            .on_action(
                cx.listener(|this, _action: &ToggleWorkspaceSidebar, _window, cx| {
                    this.toggle_workspace_sidebar(cx);
                }),
            )
    }
}
