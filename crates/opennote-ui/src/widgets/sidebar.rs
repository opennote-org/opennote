use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use gpui::{BorrowAppContext, Context, IntoElement, ParentElement, div};
use gpui_component::{
    Side,
    button::Button,
    h_flex,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};
use opennote_core_logics::note::{create_blocks, update_blocks};
use opennote_models::payload::Payload;

use crate::globals::{
    bootstrap::GlobalApplicationBootStrap,
    helpers::get_language_profile,
    states::{ProtectedBlock, States},
};

pub fn create_sidebar<T: Sized + 'static>(
    is_collapsed: bool,
    cx: &mut Context<T>,
) -> impl IntoElement {
    let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
    let states: &States = cx.global();

    if is_collapsed {
        return div();
    }

    div().child(
        Sidebar::new(Side::Left)
            .child(create_sidebar_items(states.blocks.clone()))
            .header(
                h_flex()
                    .child(language_profile.sidebar_title)
                    .child(create_new_block_button()),
            ),
    )
}

fn create_sidebar_items(blocks: Arc<RwLock<Vec<ProtectedBlock>>>) -> SidebarMenu {
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

fn create_new_block_button() -> Button {
    Button::new("workspace_sidebar_create_new_block_button")
        .label("+")
        .on_click(|click, _window, cx| {
            if !click.is_right_click() {
                let bootstrap: &GlobalApplicationBootStrap = cx.global();
                let databases = bootstrap.0.databases.clone();

                cx.spawn(async move |cx| {
                    log::debug!("Creating 1 block...");

                    let block = match create_blocks(&databases, 1).await {
                        Ok(mut result) => result.pop(),
                        Err(error) => {
                            log::error!("{}", error);
                            return Err(error);
                        },
                    };
                    
                    if let Some(block) = block {
                        todo!("Need to have a way to create payloads with proper vectors etc");
                        block.payloads.push();
                        update_blocks(&databases, vec![block]).await?
                    }

                    log::debug!("Block creation finished, preceed to refreshing the block list...");

                    let _ = cx.update_global::<States, ()>(|_this, cx| {
                        States::refresh_blocks_list(cx);
                    });

                    Ok::<(), anyhow::Error>(())
                })
                .detach();
            }
        })
}
