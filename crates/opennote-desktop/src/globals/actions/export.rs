use std::{cell::Cell, rc::Rc};

use gpui_kit::{App, SharedString, Window};
use sanitize_filename::sanitize;
use uuid::Uuid;

use opennote_models::query::BlockQuery;

use crate::globals::{
    actions::route_helpers::route_read_blocks,
    bootstrap::GlobalApplicationBootStrap,
    helpers::{get_language_profile, run_async_background},
    states::{States, server_registry::ServerStates},
    tasks::{
        helpers::start_task, task_information::TaskInformation, task_result::TaskType,
        unique_notifications::ExportNBlocksNotification,
    },
};

pub fn export_files(
    window: &mut Window,
    cx: &mut App,
    prompt: futures::channel::oneshot::Receiver<
        Result<Option<Vec<std::path::PathBuf>>, anyhow::Error>,
    >,
    blocks_to_export: Vec<Uuid>,
) {
    let language_profile = get_language_profile(cx).unwrap();
    let exporting_message = language_profile["exporting_n_blocks"].clone();
    let exported_message = language_profile["exported_n_blocks"].clone();
    let export_failed_message = language_profile["block_export_failed"].clone();
    let num_blocks = Rc::new(Cell::new(0));
    let num_blocks_for_task = num_blocks.clone();

    let task = TaskInformation::new(
        exporting_message.replace("{}", &blocks_to_export.len().to_string()),
        TaskType::ExportNBlocks,
        true,
    );

    start_task::<ExportNBlocksNotification>(
        cx,
        window,
        task,
        async move |cx, window_handle| {
            // Get the selected directory path for export
            let path = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return Ok(()),
                Ok(Err(_err)) => return Ok(()),
            };

            let (databases, system_configurations) = cx
                .read_global::<GlobalApplicationBootStrap, _>(|this, _cx| {
                    (
                        this.0.databases.clone(),
                        this.get_configurations().system.clone(),
                    )
                });

            let (server_name, server_state) = cx
                .read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window_handle.window_id())
                });

            let executor = cx.background_executor();
            let tokio_handle = tokio::runtime::Handle::current();

            let num_blocks = run_async_background(executor, tokio_handle, async move {
                let blocks = match route_read_blocks(
                    &server_name,
                    &server_state,
                    &databases,
                    &BlockQuery::ByIds(blocks_to_export),
                    false,
                    true,
                    system_configurations,
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
            .await?;

            num_blocks_for_task.set(num_blocks);

            Ok(())
        },
        |_cx| {},
        move || {
            exported_message
                .replace("{}", &num_blocks.get().to_string())
                .into()
        },
        move |error| {
            export_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );
}
