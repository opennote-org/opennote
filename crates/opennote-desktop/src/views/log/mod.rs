pub mod writer;

mod entry;

use std::{collections::VecDeque, sync::mpsc::Receiver, time::Duration};

use anyhow::Context as _;
use gpui_kit::component::{ActiveTheme, Root, Theme, v_flex};
use gpui_kit::*;

use opennote_models::constants::{
    LOADING_WINDOW_HEIGHT, LOADING_WINDOW_WIDTH, LOG_WINDOW_CAPACITY,
};

use crate::{
    globals::{helpers::get_language_profile, log::GlobalLogWindowState},
    views::log::entry::LogEntry,
    window::{create_window_option, format_window_title},
};

pub struct LogWindow {
    receiver: Receiver<String>,
    lines: VecDeque<LogEntry>,
    list_state: ListState,

    _drain_task: Task<()>,
    _subscriptions: Option<Subscription>,
}

impl LogWindow {
    pub fn install(receiver: Receiver<String>, cx: &mut App) {
        let view = cx.new(|cx: &mut Context<Self>| {
            let task = cx.spawn(async move |view, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(100))
                        .await;

                    let result = view.update(cx, |view, cx| {
                        let mut changed = false;
                        // Bound each batch so a busy producer cannot starve the UI.
                        for line in view.receiver.try_iter().take(LOG_WINDOW_CAPACITY) {
                            view.lines.push_back(LogEntry::from_ansi(&line));
                            changed = true;
                        }

                        while view.lines.len() > LOG_WINDOW_CAPACITY {
                            view.lines.pop_front();
                        }

                        if changed {
                            view.list_state.reset(view.lines.len());
                            // Start at the end; layout fills upward and clamps to
                            // the top when the entries don't fill the window yet.
                            view.list_state.scroll_to(ListOffset {
                                item_ix: view.lines.len(),
                                offset_in_item: px(0.),
                            });
                            cx.notify();
                        }
                    });

                    if result.is_err() {
                        break;
                    }
                }
            });

            Self {
                receiver,
                lines: VecDeque::new(),
                list_state: ListState::new(0, ListAlignment::Top, px(100.)),
                _drain_task: task,
                _subscriptions: None,
            }
        });
        cx.set_global(GlobalLogWindowState::new(view, None));
    }

    /// Attach window-specific state whenever the persistent log view is opened.
    fn initialize_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        Theme::sync_system_appearance(Some(window), cx);
        self._subscriptions = Some(cx.observe_window_appearance(window, |_view, window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            cx.notify();
        }));
    }

    pub fn toggle(cx: &mut App) -> anyhow::Result<()> {
        // Remove the handle if there has been a log window
        if let Some(handle) = cx.global_mut::<GlobalLogWindowState>().take_handle() {
            handle.update(cx, |_, window, _| window.remove_window())?;
            return Ok(());
        }

        // Open a new log window if there is no log window
        let language_profile = get_language_profile(cx)
            .context("Getting language profile for the log window failed")?;

        let view = cx.global::<GlobalLogWindowState>().get_view();

        let handle = cx.open_window(
            create_window_option(
                cx,
                format_window_title(Some(&language_profile["log_window_title"]), None, None),
                LOADING_WINDOW_WIDTH * 3.0,
                LOADING_WINDOW_HEIGHT,
            ),
            |window, cx| {
                view.update(cx, |view, cx| {
                    view.initialize_window(window, cx);
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )?;

        cx.global_mut::<GlobalLogWindowState>()
            .update_handle(handle);

        Ok(())
    }
}

impl Render for LogWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language_profile =
            get_language_profile(cx).expect("Getting log window language failed");
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(format!(
                        "{} · {} / {LOG_WINDOW_CAPACITY} · {}",
                        language_profile["log_window_title"],
                        self.lines.len(),
                        language_profile["log_window_newest_last"]
                    )),
            )
            .child(
                // Entries can contain newlines and wrap, so measure each one's height.
                list(
                    self.list_state.clone(),
                    cx.processor(|view, index: usize, _, cx| {
                        let highlights = view.lines[index]
                            .get_highlights()
                            .iter()
                            .map(|(range, style)| {
                                let mut style = style.clone();
                                // ANSI dim text follows the app's muted foreground in
                                // both light and dark themes; level colors stay intact.
                                if style.color == Some(rgb(0xa0a6ad).into()) {
                                    style.color = Some(cx.theme().muted_foreground);
                                }
                                (range.clone(), style)
                            })
                            .collect::<Vec<_>>();

                        div()
                            .w_full()
                            .px_2()
                            .text_sm()
                            .child(
                                StyledText::new(view.lines[index].get_text())
                                    .with_highlights(highlights),
                            )
                            .into_any_element()
                    }),
                )
                .flex_1()
                .min_h_0()
                .w_full(),
            )
    }
}
