use gpui::KeyBinding;

pub trait KeyMappingUIExtension {
    fn into_keybinding(self) -> KeyBinding;
}

pub trait KeyMappingsUIExtension {
    fn into_keybindings(self) -> Vec<KeyBinding>;
}