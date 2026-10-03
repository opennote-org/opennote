use anyhow::Result;
use async_trait::async_trait;
use gpui_kit::Action;
use tokio::sync::mpsc::Sender;

/// Implement this to initialize on app start as a resource
#[async_trait]
pub trait InitializeAsResourceOnAppStart: Sized {
    /// The message_sender is for sending messages regarding the resource loading infos.
    /// The resources will be loaded in another thread without blocking the UI.
    ///
    /// You can also register actions that you want to run after the startup flow finishes
    /// by putting them into the `dispatch_actions`.
    async fn initialize_as_resource(
        message_sender: &Sender<&'static str>,
        dispatch_actions: &Sender<Box<dyn Action>>,
    ) -> Result<Self>;
}
