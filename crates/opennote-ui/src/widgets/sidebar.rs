use std::sync::{Arc, RwLock};

use gpui::{BorrowAppContext, IntoElement, ParentElement, RenderOnce, Window};
use gpui_component::{
    Side, h_flex,
    sidebar::{Sidebar as GPUIComponentSidebar, SidebarMenu, SidebarMenuItem},
};

use opennote_models::block::Block;

use crate::globals::{helpers::get_language_profile, states::States};

#[derive(IntoElement)]
pub struct Sidebar {
    is_collapsed: bool,
}

impl Sidebar {
    pub fn new(is_collapsed: bool) -> Self {
        Self { is_collapsed }
    }

    pub fn create_sidebar_items(&self, blocks: Arc<RwLock<Vec<Block>>>) -> SidebarMenu {
        let blocks = blocks.read().unwrap();
        
        SidebarMenu::new().children(
            blocks
                .iter()
                .map(|item| {
                    SidebarMenuItem::new(item.payloads[0].texts).on_click(|click, window, cx| {
                        if !click.is_right_click() {
                            cx.update_global::<States, ()>(|states, cx| {
                                states.active_block = Some(item);
                            })
                        }
                    })
                })
                .collect(), 
        )
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();

        GPUIComponentSidebar::new(Side::Left)
            .child(self.create_sidebar_items())
            .collapsible(true)
            .collapsed(self.is_collapsed)
            .header(h_flex().child(language_profile.sidebar_title))
    }
}
