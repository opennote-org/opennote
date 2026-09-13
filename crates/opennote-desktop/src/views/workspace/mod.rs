mod actions;

use gpui::{Context, *};
use gpui_component::{Root, StyledExt, Theme, WindowExt};

use opennote_core_logics::configurations::{ApplicationType, get_configuration_folder_path};
use opennote_models::{constants::LOCAL_SERVER_NAME, traits::LoadFromAndSaveToFile};

use crate::{
    globals::{states::States, tasks::tracker::TaskTracker},
    key_mappings::key_contexts::WORKSPACE,
    views::settings::SettingsPanel,
    widgets::{
        command_bar::bar::CommandBar,
        dialogue::{PENDING_TASKS_WARNING, open_warning_dialogue},
        pane::Pane,
        search_bar::bar::SearchBar,
        sidebar::{OpenNoteSidebar, block_states::BlockStates},
    },
    window::format_window_title,
};

/// This is the root of all views in this app.
pub struct Workspace {
    focus_handle: FocusHandle,

    pub pane: Entity<Pane>,
    pub sidebar: Entity<OpenNoteSidebar>,
    pub command_bar: Entity<CommandBar>,
    pub search_bar: Entity<SearchBar>,
    pub settings_panel: Entity<SettingsPanel>,

    /// Store the previous focus.
    /// Useful when closing an UI component,
    /// and restoring the focus to a preivous one.
    pub previous_focuses: Vec<FocusHandle>,

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

        // Load block states and then create sidebar based on the block state
        let configuration_folder_path = get_configuration_folder_path(ApplicationType::Desktop);
        let block_states = BlockStates::load_from_file(configuration_folder_path)?;
        let sidebar = cx.new(|cx| OpenNoteSidebar::new(cx, block_states));

        // An editor is owned by a pane.
        // The editor creation is handled by the pane.
        let pane = cx.new(|cx| Pane::new(cx, window, sidebar.clone()));
        let editor = pane.read(cx).editor.downgrade();

        // Set the active pane and server for the workspace we have just created.
        cx.update_global::<States, ()>(|this, _cx| {
            let window_id = window.window_handle().window_id();
            this.active_panes.insert(window_id, pane.downgrade());

            let server_name = SharedString::new(LOCAL_SERVER_NAME);
            this.set_active_server(window_id, server_name.clone());

            // Set window title
            window.set_window_title(&format_window_title(None, Some(&server_name), None));
        });

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);

        // Sync the theme on workspace init
        Theme::sync_system_appearance(Some(window), cx);

        // Keep track of the system theme change.
        // The window will follow the system theme.
        _subscriptions.push(cx.observe_window_appearance(window, |_this, window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
        }));

        _subscriptions.push(cx.observe_in(&pane, window, |this, _entity, window, cx| {
            this.update_window_title(window, cx);
        }));

        _subscriptions.push(cx.observe_global_in::<States>(window, |this, window, cx| {
            this.update_window_title(window, cx);
        }));

        Ok(Self {
            focus_handle: focus_handle.clone(),
            previous_focuses: Vec::new(),
            sidebar: sidebar.clone(),
            pane,
            command_bar: cx.new(|cx| CommandBar::new(cx, window)),
            search_bar: cx.new(|cx| SearchBar::new(cx, window, editor)),
            settings_panel: cx.new(|cx| SettingsPanel::new(cx, window, sidebar.downgrade())),
            _subscriptions,
        })
    }

    /// Return the focus to a previous UI component.
    pub fn return_focus(&mut self, window: &mut Window) {
        // Return to the previously stored focus.
        // Focus on Workspace if nothing remained.
        match self.previous_focuses.pop() {
            Some(handle) => window.focus(&handle),
            None => window.focus(&self.focus_handle),
        }
    }

    /// Add new focus to the focus list.
    pub fn advance_focus(&mut self, window: &mut Window, cx: &App) {
        match window.focused(cx) {
            Some(result) => self.previous_focuses.push(result),
            None => {}
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification = Root::render_notification_layer(window, cx);
        let dialogue = Root::render_dialog_layer(window, cx);

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
            .on_action(cx.listener(Self::import_files))
            .on_action(cx.listener(Self::export_files))
            .children(notification)
            .children(dialogue)
    }
}
