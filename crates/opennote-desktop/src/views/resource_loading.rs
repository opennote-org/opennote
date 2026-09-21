use anyhow::Result;
use gpui_kit::{
    App, AppContext as _, Context, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, WindowHandle,
    component::{
        ActiveTheme as _, Sizable as _, Size, StyledExt as _, Theme, spinner::Spinner, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use crate::window::create_loading_window_option;

pub struct ResourceLoadingView {
    error_message: Option<SharedString>,

    _subscriptions: Vec<Subscription>,
}

impl ResourceLoadingView {
    pub fn open(cx: &mut App) -> Result<WindowHandle<Self>> {
        let handle = cx.open_window(create_loading_window_option(cx), |window, cx| {
            cx.new(|cx| Self::new(cx, window))
        })?;

        handle.update(cx, |_view, window, _cx| window.activate_window())?;
        Ok(handle)
    }

    pub fn new(cx: &mut Context<'_, ResourceLoadingView>, window: &mut Window) -> Self {
        // Sync the theme on init
        Theme::sync_system_appearance(Some(window), cx);

        let mut _subscriptions = Vec::new();

        // Keep track of the system theme change.
        // The window will follow the system theme.
        _subscriptions.push(cx.observe_window_appearance(window, |_this, window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
        }));

        Self {
            error_message: None,
            _subscriptions,
        }
    }

    pub fn set_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error_message = Some(message.into());
        cx.notify();
    }
}

impl Render for ResourceLoadingView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = v_flex()
            .items_center()
            .gap_3()
            .child(div().text_xl().font_semibold().child("OpenNote"))
            .when_some(self.error_message.clone(), |this, error_message| {
                this.child(
                    div()
                        .max_w_96()
                        .text_center()
                        .text_color(cx.theme().danger)
                        .child("Failed to initialize OpenNote")
                        .child(div().mt_2().text_sm().child(error_message)),
                )
            })
            .when(self.error_message.is_none(), |this| {
                this.child(Spinner::new().with_size(Size::Large)).child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading application resources…"),
                )
            });

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .p_6()
            .child(content)
    }
}
