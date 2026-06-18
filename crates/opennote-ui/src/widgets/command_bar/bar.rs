use anyhow::Context as AnyhowContext;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, ParentElement,
    Render, Styled, Subscription, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme,
    list::{List, ListState},
    v_flex,
};

use crate::{globals::helpers::get_language_profile, widgets::command_bar::keys_list::KeysList};

/// Select commands to execute
pub struct CommandBar {
    pub is_toggled: bool,
    pub keys_list: Entity<ListState<KeysList>>,

    pub focus_handle: FocusHandle,
    pub _subscriptions: Vec<Subscription>,
}

/// TODO:
/// - Create default background for editor
impl CommandBar {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        let mut _subscriptions = Vec::new();

        Self {
            is_toggled: false,
            keys_list: cx.new(|cx| ListState::new(KeysList::new(cx), window, cx).searchable(true)),
            focus_handle: cx.focus_handle(),
            _subscriptions,
        }
    }

    pub fn get_input_field_focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.keys_list.focus_handle(cx)
    }
}

impl Focusable for CommandBar {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandBar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global())
            .context("Getting language profile failed")
            .unwrap();

        div()
            .track_focus(&self.focus_handle(cx))
            .absolute()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .when(self.is_toggled, |this| this.visible())
            .when(!self.is_toggled, |this| this.invisible())
            .child(
                v_flex().child(
                    List::new(&self.keys_list)
                        .search_placeholder(language_profile.command_bar_placeholder)
                        .bg(cx.theme().accent)
                        .shadow_2xl()
                        .w_128()
                        .h_128()
                        .items_center(),
                ),
            )
        // When changes had been detected, start full text match
    }
}
