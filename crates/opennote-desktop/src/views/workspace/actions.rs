use std::io::Read;

use gpui::*;
use gpui_component::Root;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{
    configurations::fields::VectorDatabaseConfig, constants::DESKTOP_SETTINGS_PANEL_NAME,
    query::BlockQuery,
};
use sanitize_filename::sanitize;

use crate::{
    globals::{
        actions::{
            block::build_block,
            route_helpers::{route_create_blocks, route_read_blocks},
        },
        bootstrap::GlobalApplicationBootStrap,
        helpers::{get_language_profile, run_async_background},
        states::{States, helpers::get_states, server_registry::ServerStates},
        tasks::{
            task_information::TaskInformation,
            task_result::{TaskResult, TaskType},
            tracker::{register_long_running_completion, register_long_running_task},
            unique_notifications::{ExportNBlocksNotification, ImportNBlocksNotification},
        },
    },
    key_mappings::mappings::{
        CloseActiveTab, CreateOneBlock, ExportFiles, ImportFiles, NextTab, OpenNewWindow,
        PreviousTab, ToggleCommandBar, ToggleSearchBar, ToggleSettingsPanel, ToggleSidebar,
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
                self.return_focus(window);
            }

            if this.is_toggled() {
                self.advance_focus(window, cx);

                let states = get_states(cx);
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
        self.search_bar.clone().update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                self.return_focus(window);
            }

            if this.is_toggled {
                self.advance_focus(window, cx);

                let mut selected_text = None;

                let _ = this.editor.update(cx, |this, cx| {
                    let _ = this.state.update(cx, |this, cx| {
                        selected_text = this.selected_markdown_text(cx);
                    });
                });

                if let Some(query) = selected_text {
                    // Keeping newlines will cause gpui to panic in single-line rendering mode,
                    // when rendering the search input box
                    let query: String = query.lines().map(|item| item.replace("\n", " ")).collect();

                    this.search_results_list.update(cx, |this, cx| {
                        this.update_query_input_mut(cx, window, query);
                    });
                }

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
        self.command_bar.clone().update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                self.return_focus(window);
            }

            if this.is_toggled {
                self.advance_focus(window, cx);
                window.focus(&this.get_input_field_focus_handle(cx));
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

    /// Switch to the next tab in the active pane.
    pub fn next_tab(&mut self, _action: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        let states = get_states(cx);
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
        let states = get_states(cx);
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
        let language_profile = get_language_profile(cx).unwrap();
        let importing_message = language_profile["importing_n_blocks"].clone();
        let imported_message = language_profile["imported_n_blocks"].clone();
        let import_failed_message = language_profile["block_import_failed"].clone();

        // Open a dialogue to pick files
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });

        let window = window.window_handle();

        cx.spawn(async move |this, cx| {
            let paths = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(_error)) => return,
            };

            let num_blocks = paths.len();
            let task = TaskInformation::new(
                importing_message.replace("{}", &num_blocks.to_string()),
                TaskType::ImportNBlocks,
                true,
            );

            let task_id = task.id;

            // Register task in the scheduler.
            register_long_running_task::<ImportNBlocksNotification>(window, cx, task);

            let (databases, embedders, document_chunk_size, vector_database_config) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, EmbedderEntry, usize, VectorDatabaseConfig)>(
                    |this, _cx| {
                        let configurations = this.get_configurations();

                        (
                            this.0.databases.clone(),
                            this.0.embedders.clone(),
                            configurations.user.search.document_chunk_size,
                            configurations.system.vector_database.clone(),
                        )
                    },
                )
                .unwrap();

            let (server_name, server_states) = cx
                .read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window.window_id())
                })
                .unwrap();

            // Acquire a single-selected block as the imported documents' parent,
            // if any
            let mut parent_block_id = None;
            let _ = this.update(cx, |this, cx| {
                this.sidebar.update(cx, |this, cx| {
                    if let Some(tree_state) = this.get_tree_state(&server_name) {
                        tree_state.update(cx, |this, cx| {
                            parent_block_id = this.take_single_selected_block_id();
                            cx.notify();
                        });
                    }
                });
            });

            let executor = cx.background_executor();
            let tokio_handle = tokio::runtime::Handle::current();

            let mut results = Vec::new();

            for path in paths {
                let embedders = embedders.clone();

                let Some(raw_file_name) = path.file_name() else {
                    continue;
                };

                let raw_file_name = raw_file_name.to_string_lossy().to_string();

                let mut content = String::new();

                match std::fs::File::open(&path) {
                    Ok(mut file) => {
                        file.read_to_string(&mut content).unwrap();
                    }
                    Err(_error) => return,
                };

                // Create blocks for files
                let result = run_async_background(
                    executor, tokio_handle.clone(), async move {
                        build_block(
                            parent_block_id,
                            raw_file_name,
                            &embedders,
                            Some(content),
                            Some(document_chunk_size),
                        ).await
                    }
                ).await;

                results.push(result);
            }

            let mut blocks = Vec::new();
            for block in results {
                match block {
                    Ok(result) => {
                        blocks.push(result);
                    },
                    Err(error) => {
                        log::error!("Failed to build imported block: {}", error);
                        register_long_running_completion::<ImportNBlocksNotification>(
                            window,
                            cx,
                            TaskResult::new(
                                task_id,
                                false,
                                import_failed_message.replace("{}", &error.to_string()),
                                TaskType::ImportNBlocks,
                                None,
                            ),
                        );
                        return;
                    }
                }
            }

            // Store the blocks to the active server
            match route_create_blocks(
                &server_name,
                &server_states,
                &databases,
                &vector_database_config,
                blocks,
            )
            .await
            {
                Ok(_) => {}
                Err(error) => {
                    log::error!("Failed to store imported blocks: {}", error);
                    register_long_running_completion::<ImportNBlocksNotification>(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            import_failed_message.replace("{}", &error.to_string()),
                            TaskType::ImportNBlocks,
                            None,
                        ),
                    );
                    return;
                }
            }

            register_long_running_completion::<ImportNBlocksNotification>(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    imported_message.replace("{}", &num_blocks.to_string()),
                    TaskType::ImportNBlocks,
                    None,
                ),
            );

            // Refresh the sidebar
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        }).detach();
    }

    pub fn export_files(
        &mut self,
        _action: &ExportFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let language_profile = get_language_profile(cx).unwrap();
        let exporting_message = language_profile["exporting_n_blocks"].clone();
        let exported_message = language_profile["exported_n_blocks"].clone();
        let export_failed_message = language_profile["block_export_failed"].clone();

        // Open a dialogue to pick a directory
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });

        let window = window.window_handle();

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            // Get the selected directory path for export
            let path = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(_err)) => return,
            };

            let databases = cx
                .read_global::<GlobalApplicationBootStrap, Databases>(|this, _cx| {
                    this.0.databases.clone()
                })
                .unwrap();

            let (server_name, server_state) = cx
                .read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window.window_id())
                })
                .unwrap();

            let mut blocks_to_export = Vec::new();

            // Acquire all selected notes' names and contents
            let _ = this.update(cx, |this, cx| {
                this.sidebar.update(cx, |this, cx| {
                    if let Some(tree_state) = this.get_tree_state(&server_name) {
                        tree_state.update(cx, |this, cx| {
                            let block_ids = this.take_selected_block_ids();
                            blocks_to_export.extend(block_ids);

                            cx.notify();
                        });
                    }
                });
            });

            let task = TaskInformation::new(
                exporting_message.replace("{}", &blocks_to_export.len().to_string()),
                TaskType::ExportNBlocks,
                true,
            );

            let task_id = task.id;

            // Register task in the scheduler.
            register_long_running_task::<ExportNBlocksNotification>(window, cx, task);

            let executor = cx.background_executor();
            let tokio_handle = tokio::runtime::Handle::current();

            let result = run_async_background(executor, tokio_handle, async move {
                let blocks = match route_read_blocks(
                    &server_name,
                    &server_state,
                    &databases,
                    &BlockQuery::ByIds(blocks_to_export),
                    false,
                    true,
                )
                .await
                {
                    Ok(blocks) => blocks,
                    Err(error) => return Err(error),
                };

                let num_blocks = blocks.len();

                for block in blocks {
                    // Prevent filenames that include sensitive characters like / etc.
                    let title = sanitize(block.get_title());

                    // Construct save paths for each note
                    let filepath = path[0].join(title).with_extension("md");

                    // then write files
                    std::fs::write(filepath, block.get_text_content().as_bytes())?;
                }

                Ok(num_blocks)
            })
            .await;

            let num_blocks = match result {
                Ok(num_blocks) => num_blocks,
                Err(error) => {
                    log::error!("Failed to export blocks: {}", error);
                    register_long_running_completion::<ExportNBlocksNotification>(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            export_failed_message.replace("{}", &error.to_string()),
                            TaskType::ExportNBlocks,
                            None,
                        ),
                    );
                    return;
                }
            };

            register_long_running_completion::<ExportNBlocksNotification>(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    exported_message.replace("{}", &num_blocks.to_string()),
                    TaskType::ExportNBlocks,
                    None,
                ),
            );
        })
        .detach();
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
}
