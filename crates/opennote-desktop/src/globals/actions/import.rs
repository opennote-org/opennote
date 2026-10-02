use std::{cell::Cell, io::Read, rc::Rc};

use gpui_kit::{App, SharedString, Window};
use uuid::Uuid;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::configurations::system::SystemConfigurations;

use crate::globals::{
    actions::{block::build_block, route_helpers::route_create_blocks},
    bootstrap::GlobalApplicationBootStrap,
    helpers::{get_language_profile, run_async_background},
    states::{States, server_registry::ServerStates},
    tasks::{
        helpers::start_task, task_information::TaskInformation, task_result::TaskType,
        unique_notifications::ImportNBlocksNotification,
    },
};

pub fn import_files(
    window: &mut Window,
    cx: &mut App,
    parent_block_id: Option<Uuid>,
    prompt: futures::channel::oneshot::Receiver<
        Result<Option<Vec<std::path::PathBuf>>, anyhow::Error>,
    >,
) {
    let language_profile = get_language_profile(cx).unwrap();
    let importing_message = language_profile["importing_n_blocks"].clone();
    let imported_message = language_profile["imported_n_blocks"].clone();
    let import_failed_message = language_profile["block_import_failed"].clone();
    let num_blocks = Rc::new(Cell::new(0));
    let num_blocks_for_task = num_blocks.clone();

    let task = TaskInformation::new(importing_message, TaskType::ImportNBlocks, true);

    start_task::<ImportNBlocksNotification>(
        cx,
        window,
        task,
        async move |cx, window_handle| {
            let paths = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return Ok(()),
                Ok(Err(_error)) => return Ok(()),
            };

            num_blocks_for_task.set(paths.len());

            let (databases, embedders, document_chunk_size, system_configurations) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, EmbedderEntry, usize, SystemConfigurations)>(
                    |this, _cx| {
                        let configurations = this.get_configurations();

                        (
                            this.0.databases.clone(),
                            this.0.embedders.clone(),
                            configurations.user.search.document_chunk_size,
                            configurations.system.clone(),
                        )
                    },
                );

            let (server_name, server_states) = cx
                .read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window_handle.window_id())
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
                    Err(error) => return Err(error.into()),
                };

                // Create blocks for files
                let result = run_async_background(executor, tokio_handle.clone(), async move {
                    build_block(
                        parent_block_id,
                        raw_file_name,
                        &embedders,
                        Some(content),
                        Some(document_chunk_size),
                    )
                    .await
                })
                .await?;

                results.push(result);
            }

            let mut blocks = Vec::new();
            for block in results {
                blocks.push(block);
            }

            // Store the blocks to the active server
            route_create_blocks(
                &server_name,
                &server_states,
                &databases,
                &system_configurations.vector_database,
                system_configurations.clone(),
                blocks,
            )
            .await?;

            Ok(())
        },
        |cx| {
            // Refresh the sidebar
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        },
        move || {
            imported_message
                .replace("{}", &num_blocks.get().to_string())
                .into()
        },
        move |error| {
            import_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );
}
