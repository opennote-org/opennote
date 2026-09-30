pub mod traits;

use anyhow::{Result, anyhow};
use gpui_kit::*;

use opennote_models::constants::STARTUP_MESSAGE_CHANNEL_CAPACITY;

use crate::{
    globals::{
        assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap,
        helpers::run_async_background_detached, tasks::tracker::TaskTracker,
    },
    startup::traits::InitializeAsResourceOnAppStart,
    views::resource_loading::ResourceLoadingView,
};

pub fn load_frameworks(cx: &mut App) {
    // This must be called before using any GPUI Component features.
    gpui_kit::init(cx);
    // Track tasks' status
    TaskTracker::init(cx);
}

pub async fn load_resources(
    cx: &mut AsyncApp,
    resource_loading_window: WindowHandle<ResourceLoadingView>,
) -> Result<(GlobalApplicationBootStrap, AssetsCollection)> {
    let tokio_handle = tokio::runtime::Handle::current();

    // Send and receive the loaded resources
    let (resource_sender, resource_receiver) = tokio::sync::oneshot::channel();
    let (error_sender, error_receiver) = tokio::sync::oneshot::channel();

    // Send and receive the messages in streaming manner
    let (message_sender, mut message_receiver) =
        tokio::sync::mpsc::channel(STARTUP_MESSAGE_CHANNEL_CAPACITY);

    run_async_background_detached(cx.background_executor(), tokio_handle, async move {
        let bootstrap =
            match GlobalApplicationBootStrap::initialize_as_resource(&message_sender).await {
                Ok(result) => result,
                Err(error) => {
                    error_sender.send(error).unwrap();
                    return;
                }
            };

        let assets = match AssetsCollection::initialize_as_resource(&message_sender).await {
            Ok(result) => result,
            Err(error) => {
                error_sender.send(error).unwrap();
                return;
            }
        };

        let _ = resource_sender.send((bootstrap, assets));
    })
    .await;

    while let Some(message) = message_receiver.recv().await {
        let _ = resource_loading_window.update(cx, |view, _window, cx| {
            view.set_message(message, cx);
        });
    }

    match error_receiver.await {
        Ok(error) => {
            let message = format!("{error:#}");
            let _ = resource_loading_window.update(cx, |view, _window, cx| {
                view.set_error(message.clone(), cx);
            });

            return Err(anyhow!(message));
        }
        _ => {}
    };

    let (bootstrap, assets) = match resource_receiver.await {
        Ok(resources) => resources,
        Err(error) => {
            let message = format!("{error:#}");
            let _ = resource_loading_window.update(cx, |view, _window, cx| {
                view.set_error(message.clone(), cx);
            });

            return Err(anyhow!(message));
        }
    };

    Ok((bootstrap, assets))
}
