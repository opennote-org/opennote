use gpui::*;
use gpui_component::{
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    *,
};

use crate::{globals::UIApplicationBootStrap, key_mappings::mappings::ToggleWorkspaceSidebar};

pub struct MainWindow {
    focus_handler: FocusHandle,
    is_workspace_sidebar_collapsed: bool,
}

/// GPUI needs to have this trait implemented if it needs
/// to have action binding
impl Focusable for MainWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handler.clone()
    }
}

impl MainWindow {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handler: cx.focus_handle(),
            is_workspace_sidebar_collapsed: false,
        }
    }

    pub fn toggle_workspace_sidebar(
        &mut self,
        _: &ToggleWorkspaceSidebar,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            .id("workspace_sidebar")
            .key_context("workspace_sidebar")
            .h_full()
            .track_focus(&self.focus_handler) // GPUI needs this to get the focus of this Window
            .child(self.render_sidebar())
            .on_action(cx.listener(Self::toggle_workspace_sidebar))
    }
}
