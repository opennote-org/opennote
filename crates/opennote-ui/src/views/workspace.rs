use anyhow::Context as AnyhowContext;
use gpui::{Context, *};
use gpui_component::{
    Root, StyledExt, WindowExt,
    input::{InputEvent, InputState},
    notification::NotificationType,
};

use crate::{
    globals::{helpers::get_language_profile, states::States},
    key_mappings::{
        key_contexts::WORKSPACE,
        mappings::{ToggleSearchBar, ToggleSidebar},
    },
    widgets::{
        notifications::NotificationCenter,
        pane::{pane::Pane, pane_group::PaneGroup},
        search_bar::create_search_bar,
        sidebar::OpenNoteSidebar,
    },
};

/// This is the root of all views in this app.
pub struct Workspace {
    focus_handle: FocusHandle,

    pub sidebar: Entity<OpenNoteSidebar>,
    pub pane_group: Entity<PaneGroup>,
    pub notification_center: Entity<NotificationCenter>,

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

        let workspace_weak_entity = cx.weak_entity();

        Ok(Self {
            focus_handle: cx.focus_handle(),
            sidebar: cx.new(|cx| OpenNoteSidebar::new(cx, workspace_weak_entity)),
            pane_group: cx.new(|pane_group_cx| {
                let pane_group = pane_group_cx.weak_entity();
                let pane_entity = pane_group_cx.new(|cx| Pane::new(cx, window, pane_group));

                // Set the active pane to be the one we have just created,
                // so we don't have empty PaneGroup
                pane_group_cx.update_global::<States, ()>(|this, _cx| {
                    this.active_pane = Some(pane_entity.downgrade());
                });

                PaneGroup::new(pane_entity)
            }),
            notification_center: cx.new(|cx| NotificationCenter::new(cx, window)),
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
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_row() // To display items in rows
                    .child(self.sidebar.clone()) // Left
                    .child(self.pane_group.clone()), // Right
            )
            .child(create_search_bar(
                &self.search_query,
                self.is_search_bar_toggled,
            ))
            .on_action(
                cx.listener(|workspace, _action: &ToggleSidebar, window, cx| {
                    workspace.sidebar.update(cx, |this, cx| {
                        this.toggle(cx);

                        if !this.is_toggled() {
                            window.focus(&workspace.focus_handle(cx));
                        }

                        if this.is_toggled() {
                            window.focus(&this.get_tree_focus_handle(cx));
                        }
                    });

                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|workspace, _action: &ToggleSearchBar, _window, cx| {
                    // TODO: make an independent widget for search bar
                    // TODO: make search bar focus right
                    workspace.is_search_bar_toggled = !workspace.is_search_bar_toggled;
                    cx.notify();
                }),
            )
            .children(notification)
    }
}
