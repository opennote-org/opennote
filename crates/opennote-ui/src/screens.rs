use gpui::*;
use gpui_component::{
    sidebar::{Sidebar, SidebarMenu},
    *,
};

use crate::globals::UIApplicationBootStrap;

pub struct MainWindow;

impl Render for MainWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = Sidebar::new(Side::Left)
            .child(SidebarMenu::new())
            .collapsible(true);
        // TODO: we are able to access the global states from here
        let services_and_resources: &UIApplicationBootStrap = cx.global();

        v_flex().id("workspace-sidebar").h_full().child(sidebar)
    }
}
