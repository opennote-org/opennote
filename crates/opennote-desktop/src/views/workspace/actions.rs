use gpui::*;
use gpui_component::Root;
use opennote_core_logics::helpers::run_async_code;
use opennote_mcp_server::run_mcp_server;

use crate::{
    globals::{bootstrap::GlobalApplicationBootStrap, mcp_server::DesktopMCPServer, states::States},
    key_mappings::mappings::{
        CloseActiveTab, CreateOneBlock, NextTab, OpenNewWindow, PreviousTab, ToggleCommandBar,
        ToggleMCPServer, ToggleSearchBar, ToggleSettingsPanel, ToggleSidebar,
    },
    libs::theme::adapt_theme_to_system,
};

use super::Workspace;

impl Workspace {
    /// Toggle the sidebar visibility and shift focus accordingly.
    pub fn toggle_sidebar(
        &mut self,
        _action: &ToggleSidebar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            this.toggle(cx);

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled() {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled() {
                let states: &States = cx.global();
                let active_server =
                    states.get_active_server_name(window.window_handle().window_id());
                if let Some(tree_state) = this.get_tree_focus_handle(cx, &active_server) {
                    window.focus(&tree_state);
                }
            }
        });

        cx.notify();
    }

    /// Toggle the search bar and shift focus accordingly.
    pub fn toggle_search_bar(
        &mut self,
        _action: &ToggleSearchBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.search_bar.update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled {
                window.focus(&this.get_input_field_focus_handle(cx));
            }
        });

        cx.notify();
    }

    /// Toggle the command bar and shift focus accordingly.
    pub fn toggle_command_bar(
        &mut self,
        _action: &ToggleCommandBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_bar.update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled {
                window.focus(&this.get_input_field_focus_handle(cx));
            }
        });

        cx.notify();
    }

    /// Toggle the command bar and shift focus accordingly.
    pub fn toggle_mcp_server(
        &mut self,
        _action: &ToggleMCPServer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // TODO:
        // - Implement toggle mcp server

        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let mcp_server: &DesktopMCPServer = cx.global();

        let configurations =
            run_async_code(async { bootstrap.0.configurations.lock().await.user.mcp_server });

        run_mcp_server(&configurations.get_mcp_server_address(), mcp_implementation);
    }

    /// Create a new block in the active server's tree.
    pub fn create_one_block(
        &mut self,
        _action: &CreateOneBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            let states: &States = cx.global();
            let active_server = states.get_active_server_name(window.window_handle().window_id());
            let tree_state = this.get_tree_state(&active_server);

            if let Some(tree_state) = tree_state {
                this.handle_block_creation(window, cx, tree_state);
            }
        })
    }

    /// Open the settings panel in a new window.
    pub fn toggle_settings_panel(
        &mut self,
        _action: &ToggleSettingsPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings_panel = self.settings_panel.clone();
        let _ = cx
            .open_window(WindowOptions::default(), |_this, cx| {
                cx.new(|cx| Root::new(settings_panel, window, cx))
            })
            .unwrap();
    }

    /// Switch to the next tab in the active pane.
    pub fn next_tab(&mut self, _action: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        let states: &States = cx.global();
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            this.activate_next_tab(cx);
        });
    }

    /// Switch to the previous tab in the active pane.
    pub fn previous_tab(
        &mut self,
        _action: &PreviousTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let states: &States = cx.global();
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            this.activate_previous_tab(cx);
        });
    }

    /// Open a new workspace window.
    pub fn open_new_window(
        &mut self,
        _action: &OpenNewWindow,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.open_window(WindowOptions::default(), |window, cx| {
            adapt_theme_to_system(cx);

            let view = cx.new(|cx| {
                let workspace =
                    Workspace::new(window, cx).expect("Workspace initialization failed");
                workspace
            });

            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("Failed to open window");
    }

    /// Close the active tab in the active pane.
    pub fn close_active_tab(
        &mut self,
        _action: &CloseActiveTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let states: &States = cx.global();
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            if let Some(selected_block_id) = this.selected_block_id {
                this.close_tab(&selected_block_id, cx);
            }
        });
    }
}
