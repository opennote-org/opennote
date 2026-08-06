use gpui::Window;

use opennote_core_logics::payload::convert_string_to_payloads;
use opennote_models::{block::Block, payload::Payload};

use crate::globals::{
    bootstrap::GlobalApplicationBootStrap,
    tasks::{
        task_information::TaskInformation,
        task_result::{TaskResult, TaskType},
        tracker::{register_long_running_result, register_long_running_task},
        unique_notifications::ChunkBlockNotification,
    },
};

pub fn chunk_block(window: &mut Window, app_cx: &mut gpui::App, mut block: Block, text: String) {
    let bootstrap: &GlobalApplicationBootStrap = app_cx.global();
    let configurations = bootstrap.get_configurations();

    let text_chunk_size = configurations.user.search.document_chunk_size;
    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            let task = TaskInformation::new(
                "Chunking a block",
                TaskType::ChunkBlock { block_id: block.id },
                true,
            );
            let task_id = task.id;

            // Register task in the scheduler.
            register_long_running_task::<ChunkBlockNotification>(window, cx, task);

            // Chunk in the background
            let payloads: Vec<Payload> = match cx
                .background_executor()
                .spawn(async move {
                    let payloads =
                        match convert_string_to_payloads(block.id, Some(text_chunk_size), text) {
                            Ok(results) => results,
                            Err(error) => {
                                log::error!("Error when trying to save a document: {}", error);
                                return Ok(vec![]);
                            }
                        };

                    Ok::<Vec<Payload>, anyhow::Error>(payloads)
                })
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    register_long_running_result::<ChunkBlockNotification>(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            format!("Chunking failed: {}", error),
                            TaskType::ChunkBlock { block_id: block.id },
                            None,
                        ),
                    );
                    return;
                }
            };

            block.payloads = payloads;

            // Scheduler should receive the result at this point
            register_long_running_result::<ChunkBlockNotification>(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    "Chunking completed",
                    TaskType::ChunkBlock { block_id: block.id },
                    Some(serde_json::to_value(block).unwrap()),
                ),
            );
        })
        .detach();
}
