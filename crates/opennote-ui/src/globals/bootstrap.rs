use anyhow::Context as AnyhowContext;
use gpui::{App, Global};

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::configurations::Configurations;

/// This is a wrapper for ApplicationBootStrap
/// We don't want to implement the UI specific trait for the object itself
pub struct GlobalApplicationBootStrap(pub ApplicationBootStrap);

impl Global for GlobalApplicationBootStrap {}

impl GlobalApplicationBootStrap {
    pub fn init(cx: &mut App, configurations: Configurations) {
        cx.spawn(async move |cx| {
            let bootstrap = ApplicationBootStrap::new(configurations).await.unwrap();
            cx.update(|cx| {
                cx.set_global(GlobalApplicationBootStrap(bootstrap));
            })
            .context("Failed to set global state for ApplicationBootStrap")
            .unwrap();
        })
        .detach();
    }
}
