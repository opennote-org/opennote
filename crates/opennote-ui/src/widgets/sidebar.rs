use std::sync::{Arc, RwLock};

use gpui::{
    AppContext, BorrowAppContext, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    Side,
    button::Button,
    h_flex,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};

use crate::{
    globals::{
        actions::create_one_block,
        helpers::get_language_profile,
        states::{ProtectedBlock, States},
    },
    key_mappings::{key_contexts::SIDEBAR, mappings::CreateOneBlock},
};

pub struct OpenNoteSidebar {
    focus_handle: FocusHandle,
    is_toggled: bool,
}

impl OpenNoteSidebar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(), // obtain a new focus from the global pool for this view
            is_toggled: true,
        }
    }
    
    pub fn is_toggled(&self) -> bool {
        self.is_toggled
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.is_toggled = !self.is_toggled;
        cx.notify();
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
                    create_one_block(app_cx);
                }
            })
    }
}

impl Focusable for OpenNoteSidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OpenNoteSidebar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
        let states: &States = cx.global();

        div()
            .key_context(SIDEBAR)
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .when(self.is_toggled, |this| this.visible())
            .when(!self.is_toggled, |this| this.invisible())
            .child(
                Sidebar::new(Side::Left)
                    .child(Self::create_sidebar_items(states.blocks.clone()))
                    .header(
                        h_flex()
                            .child(language_profile.sidebar_title)
                            .child(Self::create_new_block_button()),
                    ),
            )
            .on_action(cx.listener(|_this, _action: &CreateOneBlock, _window, cx| {
                create_one_block(cx);
                cx.notify();
            }))
    }
}
