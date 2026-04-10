use anyhow::Context as AnyhowContext;
use gpui::{Context, prelude::FluentBuilder, *};
use gpui_component::{
    Root, StyledExt, WindowExt,
    input::{InputEvent, InputState},
    notification::NotificationType,
};

use crate::{
    globals::{actions::create_one_block, helpers::get_language_profile},
    key_mappings::{
        key_contexts::WORKSPACE,
        mappings::{CreateOneBlock, ToggleSearchBar, ToggleSidebar},
    },
    widgets::{search_bar::create_search_bar, sidebar::create_sidebar},
};

pub struct Workspace {
    focus_handle: FocusHandle,

    is_sidebar_toggled: bool,

    search_query: Entity<InputState>,
    search_query_text: SharedString,
    is_search_bar_toggled: bool,

    is_initialization_succeeded: bool,

    _subscriptions: Vec<Subscription>,
}

/// GPUI needs to have this trait implemented if it needs
/// to have action bindings
impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Result<Self> {
        let language_profile = get_language_profile(cx.global(), cx.global())
            .context("Getting language profile failed")?;

        let search_query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(&language_profile.search_bar_placeholder)
        });

        let mut _subscriptions = vec![];

        // Reserved for capturing search queries
        _subscriptions.push(cx.subscribe_in(&search_query, window, {
            let search_query = search_query.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = search_query.read(cx).value();
                    this.search_query_text = format!("{}", value).into();
                    cx.notify()
                }
                _ => {}
            }
        }));

        Ok(Self {
            focus_handle: cx.focus_handle(),
            search_query,
            search_query_text: "".into(),
            is_search_bar_toggled: false,
            is_sidebar_toggled: false,
            is_initialization_succeeded: false,
            _subscriptions,
        })
    }

    pub fn publish_initialization_successful_message(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_initialization_succeeded {
            window.push_notification(
                (
                    NotificationType::Success,
                    "Embedder model has been loaded successfully",
                ),
                cx,
            );
            self.is_initialization_succeeded = true;
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification = Root::render_notification_layer(window, cx);

        self.publish_initialization_successful_message(window, cx);

        div()
            .key_context(WORKSPACE)
            .track_focus(&self.focus_handle) // GPUI needs this to get the focus of this workspace
            .v_flex()
            .h_full()
            .child(create_search_bar(
                &self.search_query,
                self.is_search_bar_toggled,
            ))
            .child(create_sidebar(self.is_sidebar_toggled, cx))
            .on_action(
                cx.listener(|workspace, _action: &ToggleSidebar, _window, cx| {
                    workspace.is_sidebar_toggled = !workspace.is_sidebar_toggled;
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|workspace, _action: &ToggleSearchBar, _window, cx| {
                    workspace.is_search_bar_toggled = !workspace.is_search_bar_toggled;
                    cx.notify();
                }),
            )
            .when(self.is_sidebar_toggled, |this| {
                this.on_action(
                    cx.listener(|_workspace, _action: &CreateOneBlock, _window, cx| {
                        create_one_block(cx);
                        cx.notify();
                    }),
                )
            })
            .children(notification)
    }
}
