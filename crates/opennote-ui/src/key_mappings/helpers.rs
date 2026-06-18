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
