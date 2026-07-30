use gpui::{Context, *};
use gpui_component::{Root, StyledExt, WindowExt, notification::NotificationType};
use opennote_models::constants::LOCAL_SERVER_NAME;

mod actions;

use crate::{
    globals::{states::States, tasks::tracker::TaskTracker},
    key_mappings::key_contexts::WORKSPACE,
    views::settings::SettingsPanel,
    widgets::{
        command_bar::bar::CommandBar,
        dialogue::{PENDING_TASKS_WARNING, open_warning_dialogue},
        pane::Pane,
        search_bar::bar::SearchBar,
        sidebar::OpenNoteSidebar,
    },
};

/// This is the root of all views in this app.
pub struct Workspace {
    focus_handle: FocusHandle,

    pub pane: Entity<Pane>,
    pub sidebar: Entity<OpenNoteSidebar>,
    pub command_bar: Entity<CommandBar>,
    pub search_bar: Entity<SearchBar>,
    pub settings_panel: Entity<SettingsPanel>,

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

        let sidebar = cx.new(|cx| OpenNoteSidebar::new(cx));
        let pane = cx.new(|cx| Pane::new(cx, window, sidebar.clone()));

        // Set the active pane and server for the workspace we have just created.
        cx.update_global::<States, ()>(|this, _cx| {
            let window_id = window.window_handle().window_id();
            this.active_panes.insert(window_id, pane.downgrade());
            this.set_active_server(window_id, SharedString::new(LOCAL_SERVER_NAME));
        });

        Ok(Self {
            focus_handle: cx.focus_handle(),
            sidebar: sidebar.clone(),
            pane,
            command_bar: cx.new(|cx| CommandBar::new(cx, window)),
            search_bar: cx.new(|cx| SearchBar::new(cx, window)),
            settings_panel: cx.new(|cx| SettingsPanel::new(cx, window, sidebar.downgrade())),
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
        let dialogue = Root::render_dialog_layer(window, cx);

        self.publish_initialization_successful_message(window, cx);

        // Prevent the window from being closed when it has ongoing tasks.
        let window_id = window.window_handle().window_id();
        window.on_window_should_close(cx, move |this, cx| {
            let task_tracker: &TaskTracker = cx.global();
            if task_tracker.has_pending_items(window_id) {
                this.open_dialog(cx, open_warning_dialogue(PENDING_TASKS_WARNING));
                return false;
            }

            true
        });

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
                    .child(self.pane.clone()), // Right
            )
            .child(self.command_bar.clone())
            .child(self.search_bar.clone())
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::toggle_search_bar))
            .on_action(cx.listener(Self::toggle_command_bar))
            .on_action(cx.listener(Self::create_one_block))
            .on_action(cx.listener(Self::toggle_settings_panel))
            .on_action(cx.listener(Self::next_tab))
            .on_action(cx.listener(Self::previous_tab))
            .on_action(cx.listener(Self::close_active_tab))
            .on_action(cx.listener(Self::open_new_window))
            .children(notification)
            .children(dialogue)
    }
}
