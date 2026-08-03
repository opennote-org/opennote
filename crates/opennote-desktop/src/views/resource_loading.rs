use anyhow::Result;
use gpui::{
    App, AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Window, WindowBounds, WindowHandle, WindowOptions, div,
    prelude::FluentBuilder as _, px, size,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _, Size, StyledExt as _, spinner::Spinner, v_flex,
};

use crate::libs::theme::adapt_theme_to_system;

const LOADING_WINDOW_WIDTH: f32 = 420.;
const LOADING_WINDOW_HEIGHT: f32 = 240.;

pub struct ResourceLoadingView {
    error_message: Option<SharedString>,
}

impl ResourceLoadingView {
    pub fn open(cx: &mut App) -> Result<WindowHandle<Self>> {
        let bounds = Bounds::centered(
            None,
            size(px(LOADING_WINDOW_WIDTH), px(LOADING_WINDOW_HEIGHT)),
            cx,
        );
        let handle = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            |_window, cx| {
                adapt_theme_to_system(cx);
                cx.new(|_| Self::new())
            },
        )?;

        handle.update(cx, |_view, window, _cx| window.activate_window())?;
        Ok(handle)
    }

    pub fn new() -> Self {
        Self {
            error_message: None,
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
