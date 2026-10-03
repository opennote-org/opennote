use gpui_kit::App;

use crate::globals::bootstrap::GlobalApplicationBootStrap;

pub fn get_bootstrap(cx: &App) -> &GlobalApplicationBootStrap {
    cx.global()
}

pub fn get_bootstrap_mut(cx: &mut App) -> &mut GlobalApplicationBootStrap {
    cx.global_mut()
}
