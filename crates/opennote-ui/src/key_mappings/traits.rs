use gpui::{Action, KeyBinding};

pub trait KeyMappingUIExtension {
    fn into_keybinding(self) -> KeyBinding;
}

pub trait KeyMappingsUIExtension {
    fn into_keybindings(self) -> Vec<KeyBinding>;
}

pub trait KeyBindingExtension {
    fn new_with_dyn_action(
        keystrokes: &str,
        action: Box<dyn Action>,
        context: Option<&str>,
    ) -> Self;
}
