use std::collections::HashMap;

use gpui::{Action, App, SharedString};

/// Get keystrokes of an action and then return as a SharedString.
/// Return None if no key bindings detected.
pub fn get_keystrokes_as_shared_string(cx: &App, action: Box<dyn Action>) -> Option<SharedString> {
    let keymap = cx.key_bindings();
    let keymap_ref = keymap.borrow();

    let Some(binding) = keymap_ref.bindings_for_action(action.as_ref()).last() else {
        return None;
    };

    Some(
        binding
            .keystrokes()
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .into(),
    )
}

pub fn match_action_to_language(
    language_profile: HashMap<String, String>,
    action: &Box<dyn Action>,
) -> SharedString {
    let action_name = action.name();
    language_profile[action_name].clone().into()
}
