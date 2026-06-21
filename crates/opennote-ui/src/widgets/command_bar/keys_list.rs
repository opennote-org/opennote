use gpui::{Action, App, ParentElement, SharedString, Styled, prelude::FluentBuilder};
use gpui_component::{
    IndexPath, h_flex,
    label::Label,
    list::{ListDelegate, ListItem},
};

use crate::{
    globals::helpers::get_language_profile,
    key_mappings::{
        helpers::{get_keystrokes_as_shared_string, match_action_to_language},
        mappings::{CreateOneBlock, ToggleCommandBar, ToggleSidebar},
    },
};

/// Collect all available gpui actions / key bindings in this app
pub struct KeysList {
    pub actions: Vec<(Box<dyn Action>, Option<SharedString>)>,
    pub filtered_actions: Vec<(Box<dyn Action>, Option<SharedString>)>,
    pub selected_index: Option<IndexPath>,
    pub searched: bool,
}

impl KeysList {
    pub fn new(cx: &App) -> Self {
        let actions: Vec<Box<dyn Action>> = vec![
            Box::new(ToggleSidebar),
            Box::new(ToggleCommandBar),
            Box::new(CreateOneBlock),
        ];

        let actions_keymaps: Vec<(Box<dyn Action>, Option<SharedString>)> = actions
            .into_iter()
            .map(|item| {
                let item_clone = item.boxed_clone();
                let binding = get_keystrokes_as_shared_string(cx, item);

                if let Some(binding) = binding {
                    return (item_clone, Some(binding));
                }

                (item_clone, None)
            })
            .collect();

        Self {
            actions: actions_keymaps,
            filtered_actions: Vec::new(),
            selected_index: None,
            searched: false,
        }
    }

    fn create_list_item(
        &self,
        ix: IndexPath,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
        items: &Vec<(Box<dyn Action>, Option<SharedString>)>,
    ) -> Option<ListItem> {
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();

        return items.get(ix.row).map(|(action, key_binding)| {
            let action: Box<dyn Action + 'static> = action.boxed_clone();
            let action_name = match_action_to_language(language_profile, &action);

            let content = h_flex()
                .items_center()
                .justify_between()
                .child(Label::new(action_name))
                .when_some(key_binding.clone(), |this, key_binding| {
                    this.child(Label::new(key_binding))
                });

            ListItem::new(ix)
                .selected(Some(ix) == self.selected_index)
                .child(content)
                .on_click(cx.listener(move |_this, _, window, cx| {
                    window.dispatch_action(action.boxed_clone(), cx);
                }))
        });
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
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> Option<Self::Item> {
        if self.searched {
            return self.create_list_item(ix, cx, &self.filtered_actions);
        }

        self.create_list_item(ix, cx, &self.actions)
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
        if query.is_empty() {
            self.searched = false;
            return gpui::Task::ready(());
        }

        // Filter items based on query
        self.filtered_actions = self
            .actions
            .iter()
            .filter(|(action, _key_binding)| {
                action.name().to_lowercase().contains(&query.to_lowercase())
            })
            .map(|(action, key_binding)| (action.boxed_clone(), key_binding.to_owned()))
            .collect();

        self.searched = true;

        gpui::Task::ready(())
    }
}
