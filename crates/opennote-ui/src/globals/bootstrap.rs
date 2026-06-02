use gpui::{App, Global};

use opennote_bootstrap::ApplicationBootStrap;

use crate::key_mappings::traits::KeyMappingsUIExtension;

/// This is a wrapper for ApplicationBootStrap
/// We don't want to implement the UI specific trait for the object itself
pub struct GlobalApplicationBootStrap(pub ApplicationBootStrap);

impl Global for GlobalApplicationBootStrap {}

impl GlobalApplicationBootStrap {
    pub fn init(cx: &mut App, bootstrap: ApplicationBootStrap) {
        cx.bind_keys(
            bootstrap
                .configurations
                .user
                .key_mappings
                .clone()
                .into_keybindings(),
        );

        cx.set_global(GlobalApplicationBootStrap(bootstrap));
    }
}
