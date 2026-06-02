use gpui::{DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate};
use opennote_models::configurations::key_mappings::{KeyMapping, KeyMappings};

use crate::key_mappings::{
    mappings::into_action,
    traits::{KeyBindingExtension, KeyMappingUIExtension, KeyMappingsUIExtension},
};

impl KeyMappingUIExtension for KeyMapping {
    fn into_keybinding(self) -> KeyBinding {
        let keystrokes = self.sequence.concat();
        KeyBinding::new_with_dyn_action(
            &keystrokes,
            into_action(&self.context, &self.action).unwrap(),
            Some(&self.context),
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

impl KeyBindingExtension for KeyBinding {
    fn new_with_dyn_action(
        keystrokes: &str,
        action: Box<dyn gpui::Action>,
        context: Option<&str>,
    ) -> Self {
        let context_predicate =
            context.map(|context| KeyBindingContextPredicate::parse(context).unwrap().into());
        Self::load(
            keystrokes,
            action,
            context_predicate,
            false,
            None,
            &DummyKeyboardMapper,
        )
        .unwrap()
    }
}
