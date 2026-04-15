use anyhow::Context as AnyhowContext;
use gpui::{Context, *};
use gpui_component::{
    Root, StyledExt, WindowExt,
    input::{InputEvent, InputState},
    notification::NotificationType,
};

use crate::{
    globals::helpers::get_language_profile,
    key_mappings::{
        key_contexts::WORKSPACE,
        mappings::{ToggleSearchBar, ToggleSidebar},
    },
    widgets::{search_bar::create_search_bar, sidebar::OpenNoteSidebar},
};

/// This is the root of all views in this app.
pub struct Workspace {
    focus_handle: FocusHandle,

    sidebar: Entity<OpenNoteSidebar>,

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
            sidebar: cx.new(|cx| OpenNoteSidebar::new(cx)),
            search_query,
            search_query_text: "".into(),
            is_search_bar_toggled: false,
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

        log::debug!("Refreshing the workspace...");

        div()
            .key_context(WORKSPACE)
            .track_focus(&self.focus_handle) // GPUI needs this to get the focus of this workspace
            .v_flex()
            .h_full()
            .child(create_search_bar(
                &self.search_query,
                self.is_search_bar_toggled,
            ))
            .child(self.sidebar.clone())
            .on_action(
                cx.listener(|workspace, _action: &ToggleSidebar, window, cx| {
                    workspace.sidebar.update(cx, |this, cx| {
                        this.toggle(cx);
                    });

                    let sidebar = workspace.sidebar.read(cx);

                    if !sidebar.is_toggled() {
                        window.focus(&workspace.focus_handle(cx));
                    }

                    if sidebar.is_toggled() {
                        window.focus(&sidebar.focus_handle(cx));
                    }
                }),
            )
            .on_action(
                cx.listener(|workspace, _action: &ToggleSearchBar, _window, cx| {
                    workspace.is_search_bar_toggled = !workspace.is_search_bar_toggled;
                    cx.notify();
                }),
            )
            .children(notification)
    }
}
