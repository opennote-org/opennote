use gpui::Window;
use uuid::Uuid;

use opennote_core_logics::{
    block::{create_blocks, delete_blocks, read_blocks, update_blocks},
    payload::{PayloadContentParameters, build_payload, convert_string_to_payloads},
};
use opennote_data::{Databases, database::enums::BlockQuery};
use opennote_embedder::{
    entry::EmbedderEntry,
    vectorization::{send_vectorization, vectorize},
};
use opennote_models::{
    block::Block,
    configurations::system::{EmbedderConfig, VectorDatabaseConfig},
    payload::Payload,
};

use crate::globals::{
    bootstrap::GlobalApplicationBootStrap,
    helpers::get_language_profile,
    states::States,
    tasks::{
        task_information::TaskInformation,
        task_result::{TaskResult, TaskType},
        tracker::{
            register_long_running_result, register_long_running_task, register_result,
            register_task,
        },
        unique_notifications::{ChunkBlockNotification, UpdateNBlocksNotification},
    },
};

/// TODO:
/// - Use locale for the messages
///
/// It will create one new block with a default title payload.
/// This is a normal task that will only show up in the notification center on finish.
pub fn create_one_block(
    window: &mut Window,
    app_cx: &mut gpui::App,
    parent_block_id: Option<Uuid>,
) {
    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            log::debug!("Creating 1 block...");

            let task = TaskInformation::new("Creating 1 block", TaskType::Uncategorized, false);

            let task_id = task.id;

            // Register task in the scheduler.
            register_task(window, cx, task);

            let (default_block_title, databases, embedders, vector_database_config) =
                cx.read_global::<GlobalApplicationBootStrap, (
                    String,
                    Databases,
                    Option<EmbedderEntry>,
                    VectorDatabaseConfig,
                )>(|this, cx| {
                    let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();

                    let configurations = this.get_configurations();

                    (
                        language_profile.default_block_title.clone(),
                        this.0.databases.clone(),
                        this.0.embedders.clone(),
                        configurations.system.vector_database.clone(),
                    )
                })?;

            let mut block = Block::new(parent_block_id, Vec::new());

            let payload = build_payload(
                block.id,
                PayloadContentParameters {
                    title: Some(default_block_title.to_string()),
                    ..Default::default()
                },
            )?;

            match &embedders {
                Some(embedders) => {
                    let mut vectorized_payloads =
                        send_vectorization(vec![payload], &embedders).await?;

                    if let Some(vectorized_payload) = vectorized_payloads.pop() {
                        block.payloads.push(vectorized_payload);
                    }
                }
                None => {
                    log::error!(
                        "No embedders available. Please load an embedder before proceeding"
                    );
                    register_result(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            "No embedders available. Please load an embedder before proceeding",
                            TaskType::Uncategorized,
                            None,
                        ),
                    );
                    return Err(anyhow::anyhow!("No embedders available"));
                }
            }

            let num_blocks =
                match create_blocks(&vector_database_config, &databases, vec![block]).await {
                    Ok(result) => result.len(),
                    Err(error) => {
                        log::error!("{}", error);
                        register_result(
                            window,
                            cx,
                            TaskResult::new(
                                task_id,
                                false,
                                format!("Block creation failed due to {}", error),
                                TaskType::Uncategorized,
                                None,
                            ),
                        );
                        return Err(error);
                    }
                };

            log::debug!(
                "Block creation finished for {} blocks, preceed to refreshing the block list...",
                num_blocks
            );

            register_result(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    "Created 1 block",
                    TaskType::Uncategorized,
                    None,
                ),
            );

            let _ = cx.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

/// Delete n blocks specified by their ids.
/// This is a normal task that will only show up in the notification center on finish.
pub fn delete_n_blocks(window: &mut Window, app_cx: &mut gpui::App, block_ids: Vec<Uuid>) {
    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            log::debug!("Deleting {} blocks...", block_ids.len());

            let task = TaskInformation::new(
                format!("Deleting {} blocks", block_ids.len()),
                TaskType::Uncategorized,
                false,
            );

            let task_id = task.id;
            let num_blocks = block_ids.len();

            // Register task in the scheduler.
            register_task(window, cx, task);

            let (databases, vector_database_config) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, VectorDatabaseConfig)>(
                    |this, _cx| {
                        let configurations = this.get_configurations();

                        (
                            this.0.databases.clone(),
                            configurations.system.vector_database.clone(),
                        )
                    },
                )?;

            match delete_blocks(&databases, &vector_database_config, block_ids).await {
                Ok(_) => {}
                Err(error) => {
                    log::error!("{}", error);
                    register_result(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            format!("Block deletion failed due to {}", error),
                            TaskType::Uncategorized,
                            None,
                        ),
                    );
                    return Err(error);
                }
            }

            log::debug!("Blocks deletion finished, preceed to refreshing the block list...");

            register_result(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    format!("Deleted {} blocks", num_blocks),
                    TaskType::Uncategorized,
                    None,
                ),
            );

            let _ = cx.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

/// Update n blocks supplied in the parameter.
/// This is a long running task.
/// It will remove the notification on finish.
pub fn update_n_blocks(
    window: &mut Window,
    app_cx: &mut gpui::App,
    blocks: Vec<Block>,
    with_payload_changes: bool,
) {
    log::debug!("Updating blocks: {:?}", blocks);

    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            let task = TaskInformation::new(
                format!("Updating {} blocks", blocks.len()),
                TaskType::UpdateNBlocks,
                true,
            );
            let task_id = task.id;

            // Register task in the scheduler.
            register_long_running_task::<UpdateNBlocksNotification>(window, cx, task);

            let mut blocks = blocks;
            let num_blocks = blocks.len();

            let (databases, embedders, vector_database_config, embedders_config) =
                cx.read_global::<GlobalApplicationBootStrap, (
                    Databases,
                    Option<EmbedderEntry>,
                    VectorDatabaseConfig,
                    EmbedderConfig,
                )>(|this, _cx| {
                    let configurations = this.get_configurations();

                    (
                        this.0.databases.clone(),
                        this.0.embedders.clone(),
                        configurations.system.vector_database.clone(),
                        configurations.system.embedder.clone(),
                    )
                })?;

            if with_payload_changes {
                match &embedders {
                    Some(embedders) => {
                        let executor = cx.background_executor();
                        let tokio_handle = tokio::runtime::Handle::current();
                        // TODO: make this concurrent
                        for block in blocks.iter_mut() {
                            let tokio_handle = tokio_handle.clone();

                            // Take the payloads out, and swap in a default value temporarily
                            let payloads = std::mem::take(&mut block.payloads);

                            // Cheap clone
                            let embedders = embedders.clone();
                            let embedders_config = embedders_config.clone();

                            // TODO: improve the inference speed
                            let vectorized_payloads = executor
                                .spawn(async move {
                                    tokio_handle
                                        .spawn(async move {
                                            vectorize(&embedders, &embedders_config, payloads).await
                                        })
                                        .await
                                        .unwrap()
                                })
                                .await;

                            match vectorized_payloads {
                                Ok(payloads) => block.payloads = payloads,
                                Err(error) => {
                                    // TODO: error message should not automatically closed
                                    register_long_running_result::<UpdateNBlocksNotification>(
                                        window,
                                        cx,
                                        TaskResult::new(
                                            task_id,
                                            false,
                                            format!(
                                                "Error has occurred when embedding texts: {}",
                                                error
                                            ),
                                            TaskType::UpdateNBlocks,
                                            None,
                                        ),
                                    );
                                    return Err(anyhow::anyhow!("No embedders available"));
                                }
                            }
                        }
                    }
                    None => {
                        log::error!(
                            "No embedders available. Please load an embedder before proceeding"
                        );
                        register_long_running_result::<UpdateNBlocksNotification>(
                            window,
                            cx,
                            TaskResult::new(
                                task_id,
                                false,
                                "No embedders available. Please load an embedder before proceeding",
                                TaskType::UpdateNBlocks,
                                None,
                            ),
                        );
                        return Err(anyhow::anyhow!("No embedders available"));
                    }
                }
            }

            match update_blocks(&vector_database_config, &databases, blocks).await {
                Ok(_) => {}
                Err(error) => {
                    log::error!("{}", error);
                    register_long_running_result::<UpdateNBlocksNotification>(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            format!("Block update failed due to {}", error),
                            TaskType::UpdateNBlocks,
                            None,
                        ),
                    );
                    return Err(error);
                }
            }

            log::debug!("Blocks update finished, preceed to refreshing the block list...");

            register_long_running_result::<UpdateNBlocksNotification>(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    format!("Updated {} blocks", num_blocks),
                    TaskType::UpdateNBlocks,
                    None,
                ),
            );

            let _ = cx.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

/// Update parent-children relationship.
/// This is a normal task that will only show up in the notification center on finish.
pub fn update_parent(
    window: &mut Window,
    app_cx: &mut gpui::App,
    new_parent_block_id: Option<Uuid>,
    block_ids: Vec<Uuid>,
) {
    log::debug!("Updating blocks' parent...");

    let window = window.window_handle();

    app_cx
        .spawn(async move |app| {
            let (databases, vector_database_config) = app
                .read_global::<GlobalApplicationBootStrap, (Databases, VectorDatabaseConfig)>(
                    |this, _app| {
                        let databases = this.0.databases.clone();
                        let configurations = this.get_configurations();

                        (databases, configurations.system.vector_database.clone())
                    },
                )
                .unwrap();

            let task =
                TaskInformation::new("Updating blocks' parent", TaskType::Uncategorized, false);
            let task_id = task.id;

            // Register task in the scheduler.
            register_task(window, app, task);

            let num_blocks = block_ids.len();

            match read_blocks(&databases, &BlockQuery::ByIds(block_ids)).await {
                Ok(blocks) => {
                    let blocks: Vec<Block> = blocks
                        .into_iter()
                        .map(|mut item| {
                            item.parent_id = new_parent_block_id;
                            item
                        })
                        .collect();

                    match update_blocks(&vector_database_config, &databases, blocks).await {
                        Ok(_) => {}
                        Err(error) => {
                            log::error!("{}", error);
                            register_result(
                                window,
                                app,
                                TaskResult::new(
                                    task_id,
                                    false,
                                    format!("Block parent update failed due to {}", error),
                                    TaskType::Uncategorized,
                                    None,
                                ),
                            );
                        }
                    }
                }
                Err(error) => {
                    log::error!("{}", error);
                    register_result(
                        window,
                        app,
                        TaskResult::new(
                            task_id,
                            false,
                            format!("Block parent update failed due to {}", error),
                            TaskType::Uncategorized,
                            None,
                        ),
                    );
                }
            };

            log::debug!(
                "Blocks parent id update finished, preceed to refreshing the block list..."
            );

            register_result(
                window,
                app,
                TaskResult::new(
                    task_id,
                    true,
                    format!("Updated parent for {} blocks", num_blocks),
                    TaskType::Uncategorized,
                    None,
                ),
            );

            let _ = app.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });
        })
        .detach();
}

pub fn chunk_block(window: &mut Window, app_cx: &mut gpui::App, mut block: Block, text: String) {
    log::debug!("Chunking block: {:?}", block.id);

    let bootstrap: &GlobalApplicationBootStrap = app_cx.global();
    let configurations = bootstrap.get_configurations();

    let text_chunk_size = configurations.user.search.document_chunk_size;
    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            let task =
                TaskInformation::new("Chunking a block", TaskType::ChunkBlock(block.id), true);
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
                            TaskType::ChunkBlock(block.id),
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
                    TaskType::ChunkBlock(block.id),
                    Some(serde_json::to_value(block).unwrap()),
                ),
            );
        })
        .detach();
}
