use gpui::Global;

use opennote_bootstrap::ApplicationBootStrap;

/// This is a wrapper for ApplicationBootStrap
/// We don't want to implement the UI specific trait for the object itself
pub struct UIApplicationBootStrap(pub ApplicationBootStrap);

impl Global for UIApplicationBootStrap {}
