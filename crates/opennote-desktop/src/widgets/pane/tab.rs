use std::collections::HashMap;

use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::{IconName, Selectable, Sizable};
use gpui_kit::{Context, Entity, Subscription, Window, prelude::*};
use gpui_kit::{ElementId, SharedString, WeakEntity};
use uuid::Uuid;

use crate::globals::actions::block::get_block_content;
use crate::widgets::pane::subscriptions::subscribe_editor_events;
use crate::{
    globals::states::helpers::get_states,
    libs::tabs::{drag::DraggedItem, tab::Tab, tab_bar::TabBar},
    widgets::pane::Pane,
};
use opennote_velotype::editor::Editor;

pub struct TabState {
    /// It is saved when a document has just opened.
    ///
    /// Once a text change has detected, this becomes false.
    ///
    /// Once a SaveDocument action has been successfully completed,
    /// this becomes true
    pub has_saved: bool,

    /// Store the editor in the TabState.
    pub editor: Entity<Editor>,

    /// A subscription to the state change events emitted by the editor.
    _editor_events_subscription: Subscription,
}

/// Key: block_id
/// Value: TabState
pub struct TabStates(HashMap<Uuid, TabState>);

impl TabStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn does_tab_exist(&self, block_id: &Uuid) -> bool {
        self.0.contains_key(block_id)
    }

    /// This will return a false when the tab does not exist.
    pub fn has_tab_saved(&self, block_id: &Uuid) -> bool {
        match self.0.get(block_id) {
            Some(result) => result.has_saved,
            None => false,
        }
    }

    pub fn create_tab_state(
        &mut self,
        block_id: &Uuid,
        cx: &mut Context<'_, Pane>,
        window: &mut Window,
    ) {
        let texts = get_block_content(block_id, cx).unwrap();

        let editor =
            cx.new(|cx| opennote_velotype::editor::Editor::from_markdown(cx, texts.into(), None));

        let owned_block_id = *block_id;

        self.0.insert(
            *block_id,
            TabState {
                has_saved: true,
                editor: editor.clone(),
                _editor_events_subscription: cx.subscribe_in(
                    &editor,
                    window,
                    move |view, state, event, window, cx| {
                        subscribe_editor_events(owned_block_id, view, state, event, window, cx);
                    },
                ),
            },
        );
    }

    pub fn get_tab_state_editor(&self, block_id: &Uuid) -> Option<Entity<Editor>> {
        if let Some(tab_state) = self.0.get(block_id) {
            return Some(tab_state.editor.clone());
        }

        None
    }

    pub fn update_tab_save_state(&mut self, window: &mut Window, block_id: &Uuid, has_saved: bool) {
        if let Some(tab_state) = self.0.get_mut(block_id) {
            tab_state.has_saved = has_saved;

            // As long as there's one tab remains edited,
            // the corresponding window should also remain edited.
            if !tab_state.has_saved {
                window.set_window_edited(true);
            }
        }

        self.cleanup_window_edited_state(window);
    }

    /// Check if the window is good to remove the edited state.
    fn cleanup_window_edited_state(&self, window: &mut Window) {
        let has_unsaved_contents = self
            .0
            .iter()
            .any(|(_block_id, tab_state)| !tab_state.has_saved);

        if !has_unsaved_contents {
            window.set_window_edited(false);
        }
    }

    pub fn remove_tab_state(&mut self, block_id: &Uuid, window: &mut Window) {
        self.0.remove(block_id);
        self.cleanup_window_edited_state(window);
    }

    pub fn remove_all_tab_state(&mut self, window: &mut Window) {
        self.0.clear();
        self.cleanup_window_edited_state(window);
    }
}

pub fn create_tab_bar_for_blocks(
    cx: &mut Context<'_, Pane>,
    pane_reference: WeakEntity<Pane>,
    pane_id: Uuid,
    opened_block_ids: &Vec<Uuid>,
    selected_block_id: Option<Uuid>,
    openned_tab_states: &TabStates,
) -> TabBar {
    let tabs = TabBar::new("tabs").children(opened_block_ids.iter().map(|id| {
        let id = id.clone();
        let mut selected = false;

        // If we can't get the tab state, that means the application is not synced.
        // Then we probably need to quit the app.
        if !openned_tab_states.does_tab_exist(&id) {
            panic!("Opened blocks' states dis-synced. Aborted")
        }

        // The active block is the focused block
        if let Some(selected_block_id) = &selected_block_id {
            if *selected_block_id == id {
                selected = true;
            }
        }

        // Get the title of the block
        let states = get_states(cx);
        let mut title = String::new();
        if let Some(block) = states.get_block(&id) {
            title = block.get_title();
        }

        // Construct the item for dragging
        let dragged_item = DraggedItem {
            label: Some(SharedString::from(title.clone())),
            owner_pane: Some(pane_reference.clone()),
            owner_pane_id: Some(pane_id),
            block_id: Some(id),
            ..Default::default()
        };

        let mut tab = Tab::new()
            .label(title)
            .selected(selected)
            .suffix(
                Button::new(ElementId::Name(SharedString::from(format!("close-{}", id))))
                    .icon(IconName::CircleX)
                    .ghost()
                    .xsmall()
                    .rounded(ButtonRounded::Medium)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.close_tab(&id, cx, window);
                        cx.stop_propagation();
                    })),
            )
            .on_click(
                cx.listener(move |view, event: &gpui_kit::ClickEvent, window, cx| {
                    if !event.is_right_click() {
                        view.open_or_activate_tab(id, cx, window);
                    }
                }),
            )
            .on_drag(
                dragged_item.clone(),
                move |value: &DraggedItem, _point, _window, app| app.new(|_| value.clone()),
            );

        if !openned_tab_states.has_tab_saved(&id) {
            tab = tab.prefix("⏺");
        }

        tab
    }));

    tabs
}
