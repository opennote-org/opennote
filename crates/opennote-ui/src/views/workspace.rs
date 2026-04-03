use gpui::*;
use gpui_component::StyledExt;

use crate::{
    globals::UIApplicationBootStrap,
    key_mappings::mappings::{ToggleSearchBar, ToggleSidebar},
    library::widget::traits::Widget,
    widgets::{search_bar::SearchBar, sidebar::Sidebar},
};

pub struct Workspace {
    focus_handler: FocusHandle,
    sidebar: Sidebar,
    search_bar: SearchBar,
}

/// GPUI needs to have this trait implemented if it needs
/// to have action binding
impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handler.clone()
    }
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handler: cx.focus_handle(),
            sidebar: Sidebar::new(cx),
            search_bar: SearchBar::new(cx),
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // TODO: we are able to access the global states from here
        let services_and_resources: &UIApplicationBootStrap = cx.global();
        
        dbg!("Focus from Workspace: ", window.focused(cx));
        
        div()
            .v_flex()
            .id("workspace")
            .key_context("workspace")
            .h_full()
            .track_focus(&self.focus_handler) // GPUI needs this to get the focus of this workspace
            .child(self.search_bar.create(window, cx))
            .child(self.sidebar.create(window, cx))
            .on_action(
                cx.listener(|workspace, action: &ToggleSidebar, window, cx| {
                    workspace.sidebar.toggle(action, window, cx);
                }),
            )
            .on_action(
                cx.listener(|workspace, action: &ToggleSearchBar, window, cx| {
                    workspace.search_bar.toggle(action, window, cx);
                }),
            )
    }
}
