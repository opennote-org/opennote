use gpui::{Action, App, KeyBinding, ParentElement, Styled, prelude::FluentBuilder};
use gpui_component::{
    IndexPath, h_flex,
    label::Label,
    list::{ListDelegate, ListItem},
};

use crate::key_mappings::mappings::ToggleSidebar;

/// Collect all available gpui actions / key bindings in this app
pub struct KeysList {
    pub actions: Vec<(Box<dyn Action>, Option<KeyBinding>)>,
    pub filtered_actions: Vec<(Box<dyn Action>, Option<KeyBinding>)>,
    pub selected_index: Option<IndexPath>,
}

impl KeysList {
    pub fn new(cx: &App) -> Self {
        let keymap = cx.key_bindings();
        let keymap_ref = keymap.borrow();
        let actions: Vec<Box<dyn Action>> = vec![Box::new(ToggleSidebar)];

        let actions_keymaps: Vec<(Box<dyn Action>, Option<KeyBinding>)> = actions
            .into_iter()
            .map(|item| {
                let binding = keymap_ref.bindings_for_action(item.as_ref()).last();
                if let Some(binding) = binding {
                    return (item.boxed_clone(), Some(binding.to_owned()));
                }

                (item.boxed_clone(), None)
            })
            .collect();

        Self {
            actions: actions_keymaps,
            filtered_actions: Vec::new(),
            selected_index: None,
        }
    }
}

impl ListDelegate for KeysList {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.actions.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> Option<Self::Item> {
        self.actions.get(ix.row).map(|(action, key_binding)| {
            let action = action.boxed_clone();

            let content = h_flex()
                .items_center()
                .justify_between()
                .child(Label::new(action.name()))
                .when_some(key_binding.clone(), |this, key_binding: KeyBinding| {
                    this.child(Label::new(
                        key_binding
                            .keystrokes()
                            .iter()
                            .map(|item| item.to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ))
                });

            ListItem::new(ix)
                .selected(Some(ix) == self.selected_index)
                .child(content)
                .on_click(cx.listener(move |_this, _, window, cx| {
                    window.dispatch_action(action.boxed_clone(), cx);
                }))
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> gpui::Task<()> {
        // Filter items based on query
        self.filtered_actions = self
            .actions
            .iter()
            .filter(|(action, _key_binding)| {
                action.name().to_lowercase().contains(&query.to_lowercase())
            })
            .map(|(action, key_binding)| (action.boxed_clone(), key_binding.to_owned()))
            .collect();

        gpui::Task::ready(())
    }
}
