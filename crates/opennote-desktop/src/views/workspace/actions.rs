use gpui_kit::{component::Root, *};

use opennote_models::constants::DESKTOP_SETTINGS_PANEL_NAME;

use crate::{
    globals::{
        actions::{export::export_files, import::import_files, reindex},
        states::helpers::get_states,
    },
    key_mappings::mappings::{
        CloseActiveTab, CreateOneBlock, ExportFiles, ImportFiles, NextTab, OpenNewWindow,
        PreviousTab, Reindex, ToggleCommandBar, ToggleLogWindow, ToggleSearchBar,
        ToggleSettingsPanel, ToggleSidebar,
    },
    window::{create_main_window_option, format_window_title},
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
        self.sidebar.clone().update(cx, |this, cx| {
            this.toggle(cx);

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled() {
                self.return_focus(cx, window);
            }

            if this.is_toggled() {
                let states = get_states(cx);
                let active_server =
                    states.get_active_server_name(window.window_handle().window_id());
                if let Some(tree_state) = this.get_tree_focus_handle(cx, &active_server) {
                    window.focus(&tree_state, cx);
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
        self.search_bar.clone().update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                self.return_focus(cx, window);
            }

            if this.is_toggled {
                let mut selected_text = None;

                let _ = this.pane.update(cx, |this, cx| {
                    if let Some(editor) = &this.editor {
                        let _ = editor.update(cx, |this, cx| {
                            selected_text = this.selected_markdown_text(cx);
                        });
                    }
                });

                if let Some(query) = selected_text {
                    // Keeping newlines will cause gpui to panic in single-line rendering mode,
                    // when rendering the search input box
                    let query: String = query.lines().map(|item| item.replace("\n", " ")).collect();

                    this.query_input.update(cx, |this, cx| {
                        this.set_value(query, window, cx);
                    });
                }

                window.focus(&this.get_input_field_focus_handle(cx), cx);
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
        self.command_bar.clone().update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                self.return_focus(cx, window);
            }

            if this.is_toggled {
                window.focus(&this.get_input_field_focus_handle(cx), cx);
            }
        });

        cx.notify();
    }

    /// Create a new block in the active server's tree.
    pub fn create_one_block(
        &mut self,
        _action: &CreateOneBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            let states = get_states(cx);
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
            .open_window(
                create_main_window_option(format_window_title(
                    Some(DESKTOP_SETTINGS_PANEL_NAME),
                    None,
                    None,
                )),
                |_this, cx| cx.new(|cx| Root::new(settings_panel, window, cx)),
            )
            .unwrap();
    }

    pub fn toggle_log_window(
        &mut self,
        _action: &ToggleLogWindow,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = crate::views::log::LogWindow::toggle(cx) {
            tracing::error!("Failed to open log window: {error:#}");
        }
    }

    /// Switch to the next tab in the active pane.
    pub fn next_tab(&mut self, _action: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        let states = get_states(cx);
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            this.activate_next_tab(cx, window);
        });
    }

    /// Switch to the previous tab in the active pane.
    pub fn previous_tab(
        &mut self,
        _action: &PreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let states = get_states(cx);
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            this.activate_previous_tab(cx, window);
        });
    }

    /// Open a new workspace window.
    pub fn open_new_window(
        &mut self,
        _action: &OpenNewWindow,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.open_window(
            create_main_window_option(format_window_title(None, None, None)),
            |window, cx| {
                let view = cx.new(|cx| {
                    let workspace =
                        Workspace::new(window, cx).expect("Workspace initialization failed");
                    workspace
                });

                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("Failed to open window");
    }

    /// Close the active tab in the active pane.
    pub fn close_active_tab(
        &mut self,
        _action: &CloseActiveTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let states = get_states(cx);
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            if let Some(selected_block_id) = this.selected_block_id {
                this.close_tab(&selected_block_id, cx, window);
            }
        });
    }

    pub fn import_files(
        &mut self,
        _action: &ImportFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Open a dialogue to pick files
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });

        // Acquire a single-selected block as the imported documents' parent,
        // if any
        let mut parent_block_id = None;

        let states = get_states(cx);
        let active_server_name = states.get_active_server_name(window.window_handle().window_id());

        self.sidebar.update(cx, |this, cx| {
            if let Some(tree_state) = this.get_tree_state(&active_server_name) {
                tree_state.update(cx, |this, cx| {
                    parent_block_id = this.take_single_selected_block_id();
                    cx.notify();
                });
            }
        });

        import_files(window, cx, parent_block_id, prompt);
    }

    pub fn export_files(
        &mut self,
        _action: &ExportFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Open a dialogue to pick a directory
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });

        let states = get_states(cx);
        let active_server_name = states.get_active_server_name(window.window_handle().window_id());

        let mut blocks_to_export = Vec::new();

        // Acquire all selected notes' names and contents
        self.sidebar.update(cx, |this, cx| {
            if let Some(tree_state) = this.get_tree_state(&active_server_name) {
                tree_state.update(cx, |this, cx| {
                    let block_ids = this.take_selected_block_ids();
                    blocks_to_export.extend(block_ids);

                    cx.notify();
                });
            }
        });

        export_files(window, cx, prompt, blocks_to_export);
    }

    pub fn update_window_title(&self, window: &mut Window, cx: &mut Context<'_, Workspace>) {
        // recompute the window title
        let states = get_states(cx);

        let pane = self.pane.read(cx);

        // TODO: Get the selected_block_id from the pane instead, not the editor.
        // Maybe add an annotation for standardized access
        let document_name = match &pane.selected_block_id {
            Some(block_id) => Some(states.get_block(block_id).unwrap().get_title()),
            None => None,
        };

        let server_name = match pane.selected_block_id {
            Some(block_id) => Some(
                states.get_servers_by_block_ids(&vec![block_id])[0]
                    .0
                    .to_string(),
            ),
            None => None,
        };

        let server_name = match server_name {
            Some(result) => result,
            None => states
                .get_active_server(window.window_handle().window_id())
                .0
                .to_string(),
        };

        window.set_window_title(&format_window_title(
            None,
            Some(&server_name),
            document_name.as_deref(),
        ));
    }

    pub fn rebuild_index(
        &mut self,
        _action: &Reindex,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        reindex(window, cx);
    }
}
