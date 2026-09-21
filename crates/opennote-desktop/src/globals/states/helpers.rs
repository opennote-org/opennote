use gpui_kit::App;

use crate::globals::states::States;

pub fn get_states(cx: &App) -> &States {
    cx.global()
}

pub fn get_states_mut(cx: &mut App) -> &mut States {
    cx.global_mut()
}
