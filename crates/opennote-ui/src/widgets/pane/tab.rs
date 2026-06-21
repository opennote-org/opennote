use std::collections::HashMap;

use gpui::{App, Context, prelude::*};
use gpui::{ElementId, SharedString, WeakEntity};
use gpui_component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_component::{IconName, Selectable, Sizable};
use uuid::Uuid;

use crate::globals::states::States;
use crate::libs::tabs::drag::DraggedItem;
use crate::libs::tabs::tab::Tab;
use crate::libs::tabs::tab_bar::TabBar;
use crate::widgets::pane::pane::Pane;

pub struct TabState {
    /// It is saved when a document has just opened.
    ///
    /// Once a text change has detected, this becomes false.
    ///
    /// Once a SaveDocument action has been successfully completed,
    /// this becomes true
    pub has_saved: bool,

    pub unsaved_content: Option<SharedString>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            has_saved: true,
            unsaved_content: None,
        }
    }
}

impl TabState {
    pub fn set_save_state(cx: &mut App, pane: WeakEntity<Pane>, block_id: Uuid, has_saved: bool) {
        let _ = pane.update(cx, |this, _cx| {
            let Some(tab_state) = this.opened_block_states.get_mut(&block_id) else {
                return;
            };

            tab_state.has_saved = has_saved;
        });
    }
}

pub fn create_tab_bar_for_blocks(
    cx: &mut Context<'_, Pane>,
    pane_reference: WeakEntity<Pane>,
    pane_id: Uuid,
    opened_block_ids: &Vec<Uuid>,
    selected_block_id: Option<Uuid>,
    opened_block_states: &HashMap<Uuid, TabState>,
) -> TabBar {
    let tabs = TabBar::new("tabs").children(opened_block_ids.iter().map(|id| {
        let id = id.clone();
        let mut selected = false;

        // Get the save status of the active block
        let Some(tab_state) = opened_block_states.get(&id) else {
            panic!("Opened blocks' states dis-synced. Aborted")
        };

        // The active block is the focused block
        if let Some(selected_block_id) = &selected_block_id {
            if *selected_block_id == id {
                selected = true;
            }
        }

        // Get the title of the block
        let states: &States = cx.global();
        let mut title = String::new();
        if let Some(block) = states.blocks.get(&id) {
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
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.close_tab(&id, cx);
                        let entity = cx.entity();

                        if !view.has_opened_blocks() {
                            let _ = view.pane_group.update(cx, |this, cx| {
                                this.cleanup_pane_without_tabs(entity, cx);
                            });
                        }

                        cx.stop_propagation();
                    })),
            )
            .on_click(
                cx.listener(move |view, event: &gpui::ClickEvent, _window, _cx| {
                    if !event.is_right_click() {
                        view.selected_block_id = Some(id)
                    }
                }),
            )
            .on_drag(
                dragged_item.clone(),
                move |value: &DraggedItem, _point, _window, app| app.new(|_| value.clone()),
            );

        if !tab_state.has_saved {
            tab = tab.prefix("⏺");
        }

        tab
    }));
    tabs
}
