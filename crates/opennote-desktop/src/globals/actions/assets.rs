use anyhow::{Result, anyhow};
use gpui_kit::{App, AppContext, ImageSource, SharedString, Window};

use opennote_models::{
    block::Block, configurations::system::SystemConfigurations, constants::LOCAL_SERVER_NAME,
};

use crate::globals::{
    actions::route_helpers::{route_buffering_assets, route_sending_assets},
    helpers::run_async_background,
    language::LanguageProfile,
    states::server_registry::ServerStates,
    tasks::{
        helpers::start_task,
        task_information::TaskInformation,
        task_result::TaskType,
        unique_notifications::{DownloadAssetsNotification, UploadAssetsNotification},
    },
};

/// Download assets of a block in the background.
pub fn download_block_assets(
    cx: &mut App,
    window: &mut Window,
    server_name: SharedString,
    server_states: ServerStates,
    assets_storage: opennote_data::assets::AssetsStorage,
    block: Block,
    system_configurations: SystemConfigurations,
) -> Result<(), anyhow::Error> {
    let language_profile = LanguageProfile::get_language_profile(cx)?;
    let downloading_message = language_profile
        .get_description("downloading_assets")
        .ok_or_else(|| anyhow!("Missing downloading_assets translation"))?;
    let downloaded_message = language_profile
        .get_description("assets_downloaded")
        .ok_or_else(|| anyhow!("Missing assets_downloaded translation"))?;
    let download_failed_message = language_profile
        .get_description("assets_download_failed")
        .ok_or_else(|| anyhow!("Missing assets_download_failed translation"))?;

    let tokio_handle = tokio::runtime::Handle::current();
    let window_handle = window.window_handle();
    let asset_paths: Vec<_> = block
        .get_assets_references()
        .into_iter()
        .map(|reference| assets_storage.create_asset_save_path(reference))
        .collect();

    start_task::<DownloadAssetsNotification>(
        cx,
        window,
        TaskInformation::new(downloading_message, TaskType::Uncategorized, true),
        async move |cx, _window_handle| {
            run_async_background(cx.background_executor(), tokio_handle, async move {
                route_buffering_assets(
                    &server_name,
                    &server_states,
                    &assets_storage,
                    system_configurations,
                    vec![block],
                )
                .await
            })
            .await?;

            Ok(None)
        },
        move |cx| {
            let _ = cx.update_window(window_handle, |_view, window, cx| {
                for path in &asset_paths {
                    ImageSource::from(path.as_path()).remove_asset(cx);
                }
                window.refresh();
            });
        },
        move || downloaded_message.clone(),
        move |error| {
            download_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );
    Ok(())
}

/// Upload assets of a block to the remote server in the background.
pub fn upload_block_assets(
    cx: &mut App,
    window: &mut Window,
    server_name: SharedString,
    server_states: ServerStates,
    assets_storage: opennote_data::assets::AssetsStorage,
    block: Block,
    system_configurations: SystemConfigurations,
) -> Result<(), anyhow::Error> {
    if server_name == LOCAL_SERVER_NAME || block.get_assets_references().is_empty() {
        return Ok(());
    }

    let language_profile = LanguageProfile::get_language_profile(cx)?;
    let uploading_message = language_profile
        .get_description("uploading_assets")
        .ok_or_else(|| anyhow!("Missing uploading_assets translation"))?;
    let uploaded_message = language_profile
        .get_description("assets_uploaded")
        .ok_or_else(|| anyhow!("Missing assets_uploaded translation"))?;
    let upload_failed_message = language_profile
        .get_description("assets_upload_failed")
        .ok_or_else(|| anyhow!("Missing assets_upload_failed translation"))?;

    let tokio_handle = tokio::runtime::Handle::current();

    start_task::<UploadAssetsNotification>(
        cx,
        window,
        TaskInformation::new(uploading_message, TaskType::Uncategorized, true),
        async move |cx, _window_handle| {
            run_async_background(cx.background_executor(), tokio_handle, async move {
                route_sending_assets(
                    &server_name,
                    &server_states,
                    &assets_storage,
                    system_configurations,
                    vec![block],
                )
                .await
            })
            .await?;

            Ok(None)
        },
        |_cx| {},
        move || uploaded_message.clone(),
        move |error| {
            upload_failed_message
                .replace("{}", &error.to_string())
                .into()
        },
    );

    Ok(())
}
