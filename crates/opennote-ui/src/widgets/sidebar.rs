use std::sync::{Arc, RwLock};

use gpui::{BorrowAppContext, Context, IntoElement, ParentElement, Styled, div};
use gpui_component::{
    Side,
    button::Button,
    h_flex,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};
use opennote_core_logics::{
    block::{create_blocks, update_blocks},
    payload::{PayloadContentParameters, create_payload},
};
use opennote_data::Databases;
use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};

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

    div().size_full().child(
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
        .on_click(move |click, _window, app_cx| {
            if !click.is_right_click() {
                app_cx
                    .spawn(async move |cx| {
                        log::debug!("Creating 1 block...");

                        let (default_block_title, databases, embedders) = cx
                            .read_global::<GlobalApplicationBootStrap, (String, Databases, Option<EmbedderEntry>)>(
                                |this, cx| {
                                    let language_profile =
                                        get_language_profile(cx.global(), cx.global()).unwrap();

                                    (language_profile.default_block_title.clone(), this.0.databases.clone(), this.0.embedders.clone())
                                },
                            )?;

                        let block = match create_blocks(&databases, 1).await {
                            Ok(mut result) => result.pop(),
                            Err(error) => {
                                log::error!("{}", error);
                                return Err(error);
                            }
                        };

                        if let Some(mut block) = block {
                            let payload = create_payload(
                                block.id,
                                PayloadContentParameters {
                                    title: Some(default_block_title.to_string()),
                                    ..Default::default()
                                },
                            )?;

                            match &embedders {
                                Some(embedders) => {
                                    let mut vectorized_payloads =
                                        send_vectorization(vec![payload], &embedders)
                                            .await?;

                                    if let Some(vectorized_payload) = vectorized_payloads.pop() {
                                        block.payloads.push(vectorized_payload);
                                    }
                                }
                                None => {
                                    log::error!("No embedders available. Please load an embedder before proceeding");
                                    return Err(anyhow::anyhow!("No embedders available"));
                                }
                            }

                            match update_blocks(&databases, vec![block]).await {
                                Ok(_) => {},
                                Err(error) => log::error!("Error when trying to update blocks: {}", error)
                            }
                        }

                        log::debug!(
                            "Block creation finished, preceed to refreshing the block list..."
                        );

                        let _ = cx.update_global::<States, ()>(|_this, cx| {
                            States::refresh_blocks_list(cx);
                        });

                        Ok::<(), anyhow::Error>(())
                    })
                    .detach();
            }
        })
}
