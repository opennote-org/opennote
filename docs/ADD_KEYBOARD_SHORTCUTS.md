# How to add a new keyboard shortcut?

1. Go to `opennote/crates/opennote-models/src/key_mappings/mod.rs`
2. Add `KeyMapping` entries.
3. Go to `opennote/crates/opennote-desktop/src/key_mappings/mappings.rs`
4. Add entries to one of the actions! macro. The name of the entries should match up with your added `KeyMapping`s in `opennote/crates/opennote-models/src/key_mappings/mod.rs`
5. Add your entries to `into_action` function. Again, follow the patterns there
6. Go to either `crates/opennote-desktop/src/views` or `crates/opennote-desktop/src/widgets`. It depends on where you want to add your keybaord shortcut. For example, if you are trying to add a keyboard shortcut for the sidebar, you will need to add that to `crates/opennote-desktop/src/widgets/sidebar`.
