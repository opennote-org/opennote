use gpui::KeyBinding;
use opennote_models::configurations::key_mappings::{KeyMapping, KeyMappings};

use crate::key_mappings::{mappings::into_action, traits::{KeyMappingUIExtension, KeyMappingsUIExtension}};

impl KeyMappingUIExtension for KeyMapping {
    fn into_keybinding(self) -> KeyBinding {
        let keystrokes = self.sequence.concat();
        KeyBinding::new(
            &keystrokes,
            into_action(&self.context, &self.action).unwrap(),
            Some(&self.context)
        )
    }
}

impl KeyMappingsUIExtension for KeyMappings {
    fn into_keybindings(self) -> Vec<KeyBinding> {
        self.0
            .into_iter()
            .map(|item| item.into_keybinding())
            .collect()
    }
}
