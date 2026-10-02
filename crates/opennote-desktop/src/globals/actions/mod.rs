pub mod block;
pub mod chunking;
pub mod route_helpers;

use anyhow::Context;
use gpui_kit::{SharedString, Window};
use opennote_core_logics::configurations::{
    ApplicationType, get_configuration_folder_path, get_metadata,
};
use opennote_server::{request_reindex_remote_server_blocks, send_reindexed_remote_server_blocks};
use uuid::Uuid;

use opennote_data::Databases;
use opennote_embedder::{entry::EmbedderEntry, vectorization::vectorize};
use opennote_models::{
    block::Block,
    configurations::{fields::EmbedderConfig, system::SystemConfigurations},
    constants::LOCAL_SERVER_NAME,
    query::BlockQuery,
    traits::LoadFromAndSaveToFile,
};

use crate::globals::{
    actions::block::build_block,
    bootstrap::GlobalApplicationBootStrap,
    helpers::{get_language_profile, run_async_background},
    states::{States, server_registry::ServerStates},
    tasks::{
        helpers::start_task,
        task_information::TaskInformation,
        task_result::TaskType,
        unique_notifications::{
            CreateOneBlockNotifications, DeleteNBlocksNotifications, RebuildIndexNotifications,
            UpdateNBlocksNotification, UpdateParentNotification,
        },
    },
};

/// TODO:
/// - Use locale for the messages
///
/// It will create one new block with a default title payload.
/// This is a normal task that will only show up in the notification center on finish.
pub fn create_one_block(
    window: &mut Window,
    cx: &mut gpui_kit::App,
    parent_block_id: Option<Uuid>,
) {
    let language_profile = get_language_profile(cx).unwrap();
    let default_block_title = language_profile["default_block_title"].clone();
    let creating_message = language_profile["creating_one_block"].clone();
    let created_message = language_profile["created_one_block"].clone();
    let creation_failed_message = language_profile["block_creation_failed"].clone();

    let task = TaskInformation::new(creating_message, TaskType::Uncategorized, false);

    start_task::<CreateOneBlockNotifications>(
        cx,
        window,
        task,
        async move |cx, window_handle| {
            let (default_block_title, databases, embedders, system_configurations) =
                cx.read_global::<GlobalApplicationBootStrap, (
                    String,
                    Databases,
                    EmbedderEntry,
                    SystemConfigurations,
                )>(|this, _cx| {
                    let configurations = this.get_configurations();

                    (
                        default_block_title.clone(),
                        this.0.databases.clone(),
                        this.0.embedders.clone(),
                        configurations.system.clone(),
                    )
                });

            let (server_name, server) =
                cx.read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window_handle.window_id())
                });

            let block =
                build_block(parent_block_id, default_block_title, &embedders, None, None).await?;

            route_helpers::route_create_blocks(
                &server_name,
                &server,
                &databases,
                &system_configurations.vector_database,
                system_configurations.clone(),
                vec![block],
            )
            .await?;

            Ok(())
        },
        |cx| {
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        },
        move || created_message.clone().into(),
        move |error| {
            creation_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );
}

/// Delete n blocks specified by their ids.
/// This is a normal task that will only show up in the notification center on finish.
pub fn delete_n_blocks(window: &mut Window, cx: &mut gpui_kit::App, block_ids: Vec<Uuid>) {
    let language_profile = get_language_profile(cx).unwrap();
    let deleting_message = language_profile["deleting_n_blocks"].clone();
    let deleted_message = language_profile["deleted_n_blocks"].clone();
    let deletion_failed_message = language_profile["block_deletion_failed"].clone();
    let num_blocks = block_ids.len();

    let task = TaskInformation::new(
        deleting_message.replace("{}", &num_blocks.to_string()),
        TaskType::Uncategorized,
        false,
    );

    start_task::<DeleteNBlocksNotifications>(
        cx,
        window,
        task,
        async move |cx, window_handle| {
            let (databases, system_configurations) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, SystemConfigurations)>(
                    |this, _cx| {
                        let configurations = this.get_configurations();

                        (this.0.databases.clone(), configurations.system.clone())
                    },
                );

            let (server_name, server) =
                cx.read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window_handle.window_id())
                });

            route_helpers::route_delete_blocks(
                &server_name,
                &server,
                &databases,
                &system_configurations.vector_database,
                system_configurations.clone(),
                block_ids,
            )
            .await?;

            Ok(())
        },
        |cx| {
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        },
        move || {
            deleted_message
                .replace("{}", &num_blocks.to_string())
                .into()
        },
        move |error| {
            deletion_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );
}

/// Update n blocks supplied in the parameter.
/// This is a long running task.
/// It will remove the notification on finish.
pub fn update_n_blocks(
    window: &mut Window,
    cx: &mut gpui_kit::App,
    blocks: Vec<Block>,
    server_name: SharedString,
    server_states: ServerStates,
    with_payload_changes: bool,
) {
    let language_profile = get_language_profile(cx).unwrap();
    let updating_message = language_profile["updating_n_blocks"].clone();
    let updated_message = language_profile["updated_n_blocks"].clone();
    let update_failed_message = language_profile["block_update_failed"].clone();
    let num_blocks = blocks.len();

    let task = TaskInformation::new(
        updating_message.replace("{}", &blocks.len().to_string()),
        TaskType::UpdateNBlocks,
        true,
    );

    start_task::<UpdateNBlocksNotification>(
        cx,
        window,
        task,
        async move |cx, _window_handle| {
            let mut blocks = blocks;

            let (databases, embedders, system_configurations, embedders_config) =
                cx.read_global::<GlobalApplicationBootStrap, (
                    Databases,
                    EmbedderEntry,
                    SystemConfigurations,
                    EmbedderConfig,
                )>(|this, _cx| {
                    let configurations = this.get_configurations();

                    (
                        this.0.databases.clone(),
                        this.0.embedders.clone(),
                        configurations.system.clone(),
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

                    let payloads = vectorized_payloads?;
                    block.payloads = payloads;
                }
            }

            route_helpers::route_update_blocks(
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
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        },
        move || format!("{}", updated_message.replace("{}", &num_blocks.to_string())).into(),
        move |error| {
            format!(
                "{}",
                update_failed_message.replace("{}", &error.to_string())
            )
            .into()
        },
    );
}

/// Update parent-children relationship
pub fn update_parent(
    window: &mut Window,
    cx: &mut gpui_kit::App,
    new_parent_block_id: Option<Uuid>,
    block_ids: Vec<Uuid>,
) {
    let language_profile = get_language_profile(cx).unwrap();
    let updating_message = language_profile["updating_blocks_parent"].clone();
    let updated_message = language_profile["updated_parent_for_n_blocks"].clone();
    let update_failed_message = language_profile["block_parent_update_failed"].clone();
    let num_blocks = block_ids.len();

    let task = TaskInformation::new(updating_message, TaskType::Uncategorized, false);

    start_task::<UpdateParentNotification>(
        cx,
        window,
        task,
        async move |cx, window_handle| {
            let (databases, system_configurations) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, SystemConfigurations)>(
                    |this, _app| {
                        let databases = this.0.databases.clone();
                        let configurations = this.get_configurations();

                        (databases, configurations.system.clone())
                    },
                );

            let (server_name, server) =
                cx.read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window_handle.window_id())
                });

            let blocks = route_helpers::route_read_blocks(
                &server_name,
                &server,
                &databases,
                &BlockQuery::ByIds(block_ids),
                true,
                true,
                system_configurations.clone(),
            )
            .await?;

            let blocks: Vec<Block> = blocks
                .into_iter()
                .map(|mut item| {
                    item.parent_id = new_parent_block_id;
                    item
                })
                .collect();

            route_helpers::route_update_blocks(
                &server_name,
                &server,
                &databases,
                &system_configurations.vector_database,
                system_configurations.clone(),
                blocks,
            )
            .await?;

            Ok(())
        },
        |cx| {
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        },
        move || format!("{}", updated_message.replace("{}", &num_blocks.to_string())).into(),
        move |error| {
            format!(
                "{}",
                update_failed_message.replace("{}", &error.to_string())
            )
            .into()
        },
    );
}

pub fn reindex(window: &mut Window, cx: &mut gpui_kit::App) {
    let language_profile = get_language_profile(cx).unwrap();
    let reindexing_message = language_profile["rebuilding_index"].clone();
    let reindexed_message = language_profile["rebuilt_index"].clone();
    let reindex_failed_message = language_profile["index_rebuild_failed"].clone();

    let task = TaskInformation::new(reindexing_message, TaskType::RebuildIndex, true);

    start_task::<RebuildIndexNotifications>(
        cx,
        window,
        task,
        async move |cx, _window_handle| {
            let (bootstrap, system_configurations) = cx
                .read_global::<GlobalApplicationBootStrap, _>(|this, _cx| {
                    (this.0.clone(), this.get_configurations().system.clone())
                });

            let servers = cx.read_global::<States, _>(|this, _cx| {
                this.get_servers()
                    .iter()
                    .filter(|(name, _)| name.as_ref() != LOCAL_SERVER_NAME)
                    .map(|(name, server)| (name.clone(), server.clone()))
                    .collect::<Vec<_>>()
            });

            let executor = cx.background_executor();
            let tokio_handle = tokio::runtime::Handle::current();
            let local_configurations = system_configurations.clone();

            run_async_background(executor, tokio_handle.clone(), async move {
                bootstrap
                    .databases
                    .vector_database
                    .reindex_documents(
                        &local_configurations,
                        &bootstrap.databases.database,
                        &bootstrap.embedders,
                    )
                    .await?;

                let mut metadata = get_metadata(ApplicationType::Desktop)?;
                metadata.update(&local_configurations);
                metadata.save_to_file(&get_configuration_folder_path(ApplicationType::Desktop))?;

                Ok::<(), anyhow::Error>(())
            })
            .await?;

            let client = reqwest::Client::new();
            let embedders = cx
                .read_global::<GlobalApplicationBootStrap, _>(|this, _cx| this.0.embedders.clone());

            for (name, server) in servers {
                request_reindex_remote_server_blocks(
                    &client,
                    &server.connection_string,
                    &server.password,
                    &server.shared_key,
                    system_configurations.clone(),
                )
                .await
                .context(format!("Failed to start reindexing server {name}"))?;

                let mut reindexed_blocks = Vec::new();

                loop {
                    let response = send_reindexed_remote_server_blocks(
                        &client,
                        &server.connection_string,
                        &server.password,
                        reindexed_blocks,
                        &server.shared_key,
                        system_configurations.clone(),
                    )
                    .await
                    .with_context(|| format!("Failed to reindex server {name}"))?;

                    if response.finished {
                        break;
                    }

                    reindexed_blocks = response.blocks;
                    for block in &mut reindexed_blocks {
                        let payloads = std::mem::take(&mut block.payloads);
                        let embedders = embedders.clone();
                        let embedder_config = system_configurations.embedder.clone();
                        block.payloads =
                            run_async_background(executor, tokio_handle.clone(), async move {
                                vectorize(&embedders, &embedder_config, payloads).await
                            })
                            .await
                            .with_context(|| {
                                format!("Failed to vectorize blocks from server {name}")
                            })?;
                    }
                }
            }

            Ok(())
        },
        |cx| {
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        },
        move || reindexed_message.clone().into(),
        move |error| {
            reindex_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );
}
