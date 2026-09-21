pub mod block;
pub mod chunking;
pub mod route_helpers;

use gpui_kit::{SharedString, Window};
use uuid::Uuid;

use opennote_data::Databases;
use opennote_embedder::{entry::EmbedderEntry, vectorization::vectorize};
use opennote_models::{
    block::Block,
    configurations::fields::{EmbedderConfig, VectorDatabaseConfig},
    query::BlockQuery,
};

use crate::globals::{
    actions::block::build_block,
    bootstrap::GlobalApplicationBootStrap,
    helpers::{get_language_profile, run_async_background},
    states::{States, server_registry::ServerStates},
    tasks::{
        task_information::TaskInformation,
        task_result::{TaskResult, TaskType},
        tracker::{
            register_long_running_completion, register_long_running_task, register_result,
            register_task,
        },
        unique_notifications::UpdateNBlocksNotification,
    },
};

/// TODO:
/// - Use locale for the messages
///
/// It will create one new block with a default title payload.
/// This is a normal task that will only show up in the notification center on finish.
pub fn create_one_block(
    window: &mut Window,
    app_cx: &mut gpui_kit::App,
    parent_block_id: Option<Uuid>,
) {
    let language_profile = get_language_profile(app_cx).unwrap();
    let default_block_title = language_profile["default_block_title"].clone();
    let creating_message = language_profile["creating_one_block"].clone();
    let created_message = language_profile["created_one_block"].clone();
    let creation_failed_message = language_profile["block_creation_failed"].clone();

    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            let task = TaskInformation::new(creating_message, TaskType::Uncategorized, false);

            let task_id = task.id;

            // Register task in the scheduler.
            register_task(window, cx, task);

            let (default_block_title, databases, embedders, vector_database_config) =
                cx.read_global::<GlobalApplicationBootStrap, (
                    String,
                    Databases,
                    EmbedderEntry,
                    VectorDatabaseConfig,
                )>(|this, _cx| {
                    let configurations = this.get_configurations();

                    (
                        default_block_title.clone(),
                        this.0.databases.clone(),
                        this.0.embedders.clone(),
                        configurations.system.vector_database.clone(),
                    )
                });

            let (server_name, server) =
                cx.read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window.window_id())
                });

            let block =
                build_block(parent_block_id, default_block_title, &embedders, None, None).await?;

            match route_helpers::route_create_blocks(
                &server_name,
                &server,
                &databases,
                &vector_database_config,
                vec![block],
            )
            .await
            {
                Ok(_result) => {}
                Err(error) => {
                    tracing::error!("{}", error);
                    register_result(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            creation_failed_message.replace("{}", &error.to_string()),
                            TaskType::Uncategorized,
                            None,
                        ),
                    );
                    return Err(error);
                }
            };

            register_result(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    created_message,
                    TaskType::Uncategorized,
                    None,
                ),
            );

            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

/// Delete n blocks specified by their ids.
/// This is a normal task that will only show up in the notification center on finish.
pub fn delete_n_blocks(window: &mut Window, app_cx: &mut gpui_kit::App, block_ids: Vec<Uuid>) {
    let language_profile = get_language_profile(app_cx).unwrap();
    let deleting_message = language_profile["deleting_n_blocks"].clone();
    let deleted_message = language_profile["deleted_n_blocks"].clone();
    let deletion_failed_message = language_profile["block_deletion_failed"].clone();

    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            let task = TaskInformation::new(
                deleting_message.replace("{}", &block_ids.len().to_string()),
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
                );

            let (server_name, server) =
                cx.read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window.window_id())
                });

            match route_helpers::route_delete_blocks(
                &server_name,
                &server,
                &databases,
                &vector_database_config,
                block_ids,
            )
            .await
            {
                Ok(_) => {}
                Err(error) => {
                    tracing::error!("{}", error);
                    register_result(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            deletion_failed_message.replace("{}", &error.to_string()),
                            TaskType::Uncategorized,
                            None,
                        ),
                    );
                    return Err(error);
                }
            }

            register_result(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    deleted_message.replace("{}", &num_blocks.to_string()),
                    TaskType::Uncategorized,
                    None,
                ),
            );

            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
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
    app_cx: &mut gpui_kit::App,
    blocks: Vec<Block>,
    server_name: SharedString,
    server_states: ServerStates,
    with_payload_changes: bool,
) {
    let language_profile = get_language_profile(app_cx).unwrap();
    let updating_message = language_profile["updating_n_blocks"].clone();
    let updated_message = language_profile["updated_n_blocks"].clone();
    let update_failed_message = language_profile["block_update_failed"].clone();
    let embedding_error_message = language_profile["embedding_texts_error"].clone();

    let window = window.window_handle();

    app_cx
        .spawn(async move |cx| {
            let task = TaskInformation::new(
                updating_message.replace("{}", &blocks.len().to_string()),
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
                    EmbedderEntry,
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
                });

            if with_payload_changes {
                let executor = cx.background_executor();
                let tokio_handle = tokio::runtime::Handle::current();
                // TODO: make this concurrent
                for block in blocks.iter_mut() {
                    // Take the payloads out, and swap in a default value temporarily
                    let payloads = std::mem::take(&mut block.payloads);

                    // Cheap clone
                    let embedders = embedders.clone();
                    let embedders_config = embedders_config.clone();

                    // TODO: improve the inference speed
                    let vectorized_payloads =
                        run_async_background(executor, tokio_handle.clone(), async move {
                            vectorize(&embedders, &embedders_config, payloads).await
                        })
                        .await;

                    match vectorized_payloads {
                        Ok(payloads) => block.payloads = payloads,
                        Err(error) => {
                            // TODO: error message should not automatically closed
                            register_long_running_completion::<UpdateNBlocksNotification>(
                                window,
                                cx,
                                TaskResult::new(
                                    task_id,
                                    false,
                                    embedding_error_message.replace("{}", &error.to_string()),
                                    TaskType::UpdateNBlocks,
                                    None,
                                ),
                            );
                            return Err(anyhow::anyhow!("No embedders available"));
                        }
                    }
                }
            }

            match route_helpers::route_update_blocks(
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
                    tracing::error!("{}", error);
                    register_long_running_completion::<UpdateNBlocksNotification>(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            update_failed_message.replace("{}", &error.to_string()),
                            TaskType::UpdateNBlocks,
                            None,
                        ),
                    );
                    return Err(error);
                }
            }

            register_long_running_completion::<UpdateNBlocksNotification>(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    updated_message.replace("{}", &num_blocks.to_string()),
                    TaskType::UpdateNBlocks,
                    None,
                ),
            );

            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

/// Update parent-children relationship.
/// This is a normal task that will only show up in the notification center on finish.
pub fn update_parent(
    window: &mut Window,
    app_cx: &mut gpui_kit::App,
    new_parent_block_id: Option<Uuid>,
    block_ids: Vec<Uuid>,
) {
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
                );

            let task =
                TaskInformation::new("Updating blocks' parent", TaskType::Uncategorized, false);
            let task_id = task.id;

            // Register task in the scheduler.
            register_task(window, app, task);

            let num_blocks = block_ids.len();

            let (server_name, server) =
                app.read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window.window_id())
                });

            match route_helpers::route_read_blocks(
                &server_name,
                &server,
                &databases,
                &BlockQuery::ByIds(block_ids),
                true,
                true,
            )
            .await
            {
                Ok(blocks) => {
                    let blocks: Vec<Block> = blocks
                        .into_iter()
                        .map(|mut item| {
                            item.parent_id = new_parent_block_id;
                            item
                        })
                        .collect();

                    match route_helpers::route_update_blocks(
                        &server_name,
                        &server,
                        &databases,
                        &vector_database_config,
                        blocks,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(error) => {
                            tracing::error!("{}", error);
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
                    tracing::error!("{}", error);
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

            let _ = app.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        })
        .detach();
}
