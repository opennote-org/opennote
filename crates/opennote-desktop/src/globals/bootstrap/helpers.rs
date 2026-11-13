use gpui_kit::App;
use tokio::sync::MutexGuard;

use opennote_models::configurations::desktop::DesktopConfigurations;

use crate::globals::bootstrap::GlobalApplicationBootStrap;

pub fn get_bootstrap(cx: &App) -> &GlobalApplicationBootStrap {
    cx.global()
}

pub fn get_bootstrap_mut(cx: &mut App) -> &mut GlobalApplicationBootStrap {
    cx.global_mut()
}

/// Get the desktop app's configurations.
pub fn get_configurations(cx: &App) -> MutexGuard<'_, DesktopConfigurations> {
    let bootstrap = get_bootstrap(cx);
    bootstrap.get_configurations()
}
