use std::sync::{Arc, RwLock};

use gpui::{AppContext, BorrowAppContext, Entity, IntoElement, ParentElement, RenderOnce, Window};
use gpui_component::{
    Side,
    button::Button,
    h_flex,
    plot::label,
    sidebar::{Sidebar as GPUIComponentSidebar, SidebarMenu, SidebarMenuItem},
};
use opennote_core_logics::note::create_blocks;

use crate::{
    globals::{
        bootstrap::GlobalApplicationBootStrap,
        helpers::get_language_profile,
        states::{ProtectedBlock, States},
    },
    views::workspace::Workspace,
};

#[derive(IntoElement)]
pub struct Sidebar {
    is_collapsed: bool,
}

impl Sidebar {
    pub fn new(is_collapsed: bool) -> Self {
        Self { is_collapsed }
    }

    pub fn create_sidebar_items(blocks: Arc<RwLock<Vec<ProtectedBlock>>>) -> SidebarMenu {
        let blocks = blocks.read().unwrap();

        let sidebar_menu_items: Vec<SidebarMenuItem> = blocks
            .iter()
            .map(|item| {
                let read_item = item.0.read().unwrap();

                let mut label = String::new();
                if read_item.payloads.len() != 0 {
                    label = read_item.payloads[0].texts.clone();
                }

                let active_block = item.clone();

                SidebarMenuItem::new(label).on_click(move |click, _window, cx| {
                    if !click.is_right_click() {
                        cx.update_global::<States, ()>(|states, _cx| {
                            states.active_block = Some(active_block.clone());
                        })
                    }
                })
            })
            .collect();

        SidebarMenu::new().children(sidebar_menu_items)
    }

    pub fn create_new_block_button() -> Button {
        Button::new("workspace_sidebar_create_new_block_button")
            .label("+")
            .on_click(|click, window, cx| {
                if !click.is_right_click() {
                    let bootstrap: &GlobalApplicationBootStrap = cx.global();
                    let databases = bootstrap.0.databases.clone();
                    let window_handle = window.window_handle();

                    cx.spawn(async move |cx| {
                        log::debug!("Creating 1 block...");

                        match create_blocks(&databases, 1).await {
                            Ok(_result) => {}
                            Err(error) => log::error!("{}", error),
                        }

                        cx.update_window(window_handle, |view, _window, cx| {
                            log::debug!(
                                "Block creation finished, preceed to refreshing the block list..."
                            );

                            // TODO: this will error out, think a different way
                            let workspace: Entity<Workspace> = match view.downcast() {
                                Ok(result) => result,
                                Err(error) => {
                                    log::error!("Error when getting a view: {:#?}", error);
                                    panic!()
                                }
                            };
                            
                            workspace.update(cx, |this, cx| {
                                this.refresh_blocks_list(cx);
                            });
                        })
                        .unwrap();

                        Ok::<(), anyhow::Error>(())
                    })
                    .detach();
                }
            })
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
        let states: &States = cx.global();

        GPUIComponentSidebar::new(Side::Left)
            .child(Self::create_sidebar_items(states.blocks.clone()))
            .collapsible(true)
            .collapsed(self.is_collapsed)
            .header(
                h_flex()
                    .child(language_profile.sidebar_title)
                    .child(Self::create_new_block_button()),
            )
    }
}
