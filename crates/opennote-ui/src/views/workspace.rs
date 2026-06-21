use gpui::{Context, *};
use gpui_component::{Root, StyledExt, WindowExt, notification::NotificationType};

use crate::{
    globals::{states::States, tasks::tracker::TaskTracker},
    key_mappings::{
        key_contexts::WORKSPACE,
        mappings::{CreateOneBlock, ToggleCommandBar, ToggleSearchBar, ToggleSidebar},
    },
    widgets::{
        command_bar::bar::CommandBar,
        pane::{pane::Pane, pane_group::PaneGroup},
        search_bar::bar::SearchBar,
        sidebar::OpenNoteSidebar,
    },
};

/// This is the root of all views in this app.
pub struct Workspace {
    focus_handle: FocusHandle,

    pub sidebar: Entity<OpenNoteSidebar>,
    pub pane_group: Entity<PaneGroup>,
    pub command_bar: Entity<CommandBar>,
    pub search_bar: Entity<SearchBar>,

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
        let mut _subscriptions = vec![];

        let workspace_weak_entity = cx.weak_entity();

        let sidebar = cx.new(|cx| OpenNoteSidebar::new(cx, workspace_weak_entity));

        Ok(Self {
            focus_handle: cx.focus_handle(),
            sidebar: sidebar.clone(),
            pane_group: cx.new(|pane_group_cx| {
                let entity = pane_group_cx.entity();

                let pane_entity =
                    pane_group_cx.new(|cx| Pane::new(cx, window, entity, sidebar.clone()));

                // Set the active pane to be the one we have just created,
                // so we don't have empty PaneGroup
                pane_group_cx.update_global::<States, ()>(|this, _cx| {
                    this.active_pane = Some(pane_entity.downgrade());
                });

                PaneGroup::new(pane_entity, sidebar.clone())
            }),
            command_bar: cx.new(|cx| CommandBar::new(cx, window)),
            search_bar: cx.new(|cx| SearchBar::new(cx, window)),
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

        // Prevent the window from being closed when there are tasks ongoing
        window.on_window_should_close(cx, |_this, cx| {
            let task_tracker: &TaskTracker = cx.global();
            if task_tracker.has_pending_items() {
                return false;
            }

            true
        });

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
            .child(self.command_bar.clone())
            .child(self.search_bar.clone())
            .on_action(
                cx.listener(|workspace, _action: &ToggleSidebar, window, cx| {
                    workspace.sidebar.update(cx, |this, cx| {
                        this.toggle(cx);

                        // Manually shift the focus, otherwise it won't just focus automatically
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
                cx.listener(|workspace, _action: &ToggleSearchBar, window, cx| {
                    workspace.search_bar.update(cx, |this, cx| {
                        this.is_toggled = !this.is_toggled;

                        // Manually shift the focus, otherwise it won't just focus automatically
                        if !this.is_toggled {
                            window.focus(&workspace.focus_handle(cx));
                        }

                        if this.is_toggled {
                            window.focus(&this.get_input_field_focus_handle(cx));
                        }
                    });

                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|workspace, _action: &ToggleCommandBar, window, cx| {
                    workspace.command_bar.update(cx, |this, cx| {
                        this.is_toggled = !this.is_toggled;

                        // Manually shift the focus, otherwise it won't just focus automatically
                        if !this.is_toggled {
                            window.focus(&workspace.focus_handle(cx));
                        }

                        if this.is_toggled {
                            window.focus(&this.get_input_field_focus_handle(cx));
                        }
                    });

                    cx.notify();
                }),
            )
            .on_action(cx.listener(|this, _action: &CreateOneBlock, window, cx| {
                this.sidebar.update(cx, |this, cx| {
                    this.handle_block_creation(window, cx);
                })
            }))
            .children(notification)
    }
}
