pub mod traits;

use anyhow::{Error, Result, anyhow};
use gpui_kit::*;

use opennote_models::constants::STARTUP_MESSAGE_CHANNEL_CAPACITY;

use crate::{
    globals::{
        assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap,
        tasks::tracker::TaskTracker,
    },
    startup::traits::InitializeAsResourceOnAppStart,
    windows::resource_loading::ResourceLoadingView,
};

pub fn load_frameworks(cx: &mut App) {
    // This must be called before using any GPUI Component features.
    gpui_kit::init(cx);
    // Track tasks' status
    TaskTracker::init(cx);
}

pub struct ResourceLoader {
    pub resource: tokio::sync::oneshot::Receiver<(GlobalApplicationBootStrap, AssetsCollection)>,
    pub errors: tokio::sync::oneshot::Receiver<Error>,
    pub messages: tokio::sync::mpsc::Receiver<&'static str>,
    pub actions: tokio::sync::mpsc::Receiver<Box<dyn Action + 'static>>,
}

impl ResourceLoader {
    pub async fn start_loading_resources() -> Self {
        // Send and receive the loaded resources
        let (resource_sender, resource_receiver) = tokio::sync::oneshot::channel();
        let (error_sender, error_receiver) = tokio::sync::oneshot::channel();

        // Send and receive the messages in streaming manner
        let (message_sender, message_receiver) =
            tokio::sync::mpsc::channel(STARTUP_MESSAGE_CHANNEL_CAPACITY);
        let (dispatch_action_sender, dispatch_action_receiver) =
            tokio::sync::mpsc::channel(STARTUP_MESSAGE_CHANNEL_CAPACITY);

        tokio::spawn(async move {
            let bootstrap = match GlobalApplicationBootStrap::initialize_as_resource(
                &message_sender,
                &dispatch_action_sender,
            )
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    error_sender.send(error).unwrap();
                    return;
                }
            };

            let assets = match AssetsCollection::initialize_as_resource(
                &message_sender,
                &dispatch_action_sender,
            )
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    error_sender.send(error).unwrap();
                    return;
                }
            };

            let _ = resource_sender.send((bootstrap, assets));
        });

        Self {
            resource: resource_receiver,
            errors: error_receiver,
            messages: message_receiver,
            actions: dispatch_action_receiver,
        }
    }

    /// Loop for receiving resource loading related messages
    pub async fn receive(
        mut self,
        cx: &mut AsyncApp,
        resource_loading_window: WindowHandle<ResourceLoadingView>,
        pending_actions: &mut Vec<Box<dyn Action>>,
    ) -> Result<(GlobalApplicationBootStrap, AssetsCollection)> {
        // Listen to channels
        loop {
            tokio::select! {
                Some(message) = self.messages.recv() => {
                    let _ = resource_loading_window.update(cx, |view, _window, cx| {
                        view.set_message(message, cx);
                    });
                }
                Some(action) = self.actions.recv() => {
                    pending_actions.push(action);
                }
                else => {
                    break;
                }
            }
        }

        if let Ok(error) = self.errors.await {
            let message = format!("{error:#}");
            let _ = resource_loading_window.update(cx, |view, _window, cx| {
                view.set_error(message.clone(), cx);
            });

            return Err(anyhow!(message));
        }

        let (bootstrap, assets) = match self.resource.await {
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
}
