use gpui::{
    Action, DefiniteLength, DragMoveEvent, ElementId, Entity, Focusable, Point, SharedString,
    Subscription, WeakEntity, div,
};
use gpui::{Context, FocusHandle, Render, Window, prelude::*};
use gpui_component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_component::{ActiveTheme, IconName, Selectable, Sizable};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::globals::states::States;
use crate::libs::tabs::drag::DraggedItem;
use crate::libs::tabs::tab::Tab;
use crate::libs::tabs::tab_bar::TabBar;
use crate::widgets::editor::Editor;
use crate::widgets::pane::pane_group::{PaneGroup, SplitDirection};

macro_rules! split_structs {
    ($($name:ident => $doc:literal),* $(,)?) => {
        $(
            #[doc = $doc]
            #[derive(Clone, PartialEq, Debug, Deserialize, Default, Action, JsonSchema)]
            #[action(namespace = pane)]
            #[serde(deny_unknown_fields, default)]
            pub struct $name {
                pub mode: SplitMode,
            }
        )*
    };
}

split_structs!(
    SplitLeft => "Splits the pane to the left.",
    SplitRight => "Splits the pane to the right.",
    SplitUp => "Splits the pane upward.",
    SplitDown => "Splits the pane downward.",
    SplitHorizontal => "Splits the pane horizontally.",
    SplitVertical => "Splits the pane vertically."
);

const DROP_TARGET_SIZE: f32 = 0.2;

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub enum SplitMode {
    /// Clone the current pane.
    #[default]
    ClonePane,
    /// Create an empty new pane.
    EmptyPane,
    /// Move the item into a new pane. This will map to nop if only one pane exists.
    MovePane,
}

/// A container for 0 to many items that are open in the workspace.
/// Treats all items uniformly via the [`ItemHandle`] trait, whether it's an editor, search results multibuffer, terminal or something else,
/// responsible for managing item tabs, focus and zoom states and drag and drop features.
/// Can be split, see `PaneGroup` for more details.
pub struct Pane {
    pub id: Uuid,
    focus_handle: FocusHandle,
    selected_block_id: Option<Uuid>,
    opened_block_ids: Vec<Uuid>,
    editor: Entity<Editor>,
    drag_split_direction: Option<SplitDirection>,
    pane_group: WeakEntity<PaneGroup>,

    _subscriptions: Vec<Subscription>,
}

impl Pane {
    pub fn new(
        cx: &mut Context<Self>,
        window: &mut gpui::Window,
        pane_group: WeakEntity<PaneGroup>,
    ) -> Self {
        let mut _subscriptions = Vec::new();

        Self {
            id: Uuid::new_v4(),
            focus_handle: cx.focus_handle(),
            selected_block_id: None,
            drag_split_direction: None,
            editor: cx.new(|cx| Editor::new(cx, window)),
            opened_block_ids: Vec::new(),
            pane_group,
            _subscriptions,
        }
    }

    fn close_tab(&mut self, block_id: &Uuid, cx: &mut Context<Self>) {
        // if we have multiple tabs openning
        if self.opened_block_ids.len() > 1 {
            // Remove the closed block from the openned blocks,
            // while also retain an index for moving the focus to the prevoius one
            let mut removed_index: isize = 0;
            for (index, opened_block_id) in self.opened_block_ids.iter().enumerate() {
                if opened_block_id == block_id && index != 0 {
                    removed_index = index as isize;
                    break;
                }
            }

            self.opened_block_ids.remove(removed_index as usize);

            // Move the focus to the previous tab / block
            if let Some(selected_block_id) = &self.selected_block_id {
                let mut index_to_focus = removed_index - 1;

                // Handle if the closed tab is the first one with no previous tabs
                if index_to_focus < 0 {
                    index_to_focus = 0;
                }

                let Some(block_to_be_selected) = self.opened_block_ids.get(index_to_focus as usize)
                else {
                    return;
                };

                // Move the focus only when the active block has been closed
                if selected_block_id == block_id {
                    self.selected_block_id = Some(block_to_be_selected.clone())
                }
            }

            cx.notify();

            // Prevent triggering the 1 tab case when
            // the openned tabs become 1 after the tab closing
            return;
        }

        // if we only have 1 tab openning
        if self.opened_block_ids.len() == 1 {
            self.opened_block_ids.clear();
            self.selected_block_id = None;

            cx.notify();
        }

        // no tab closing for 0 tabs
    }

    fn handle_drag_move<T: 'static>(
        &mut self,
        event: &DragMoveEvent<T>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane_area = event.bounds.size;
        let cursor_position = event.event.position;

        let relative_cursor = Point::new(
            event.event.position.x - event.bounds.left(),
            event.event.position.y - event.bounds.top(),
        );

        // Relative size of the drop target in the editor that will open dropped file as a split pane (0-0.5)
        // E.g. 0.25 == If you drop onto the top/bottom quarter of the pane a new vertical split will be used
        //              If you drop onto the left/right quarter of the pane a new horizontal split will be used
        //
        // Referenced from Zed editor's default.json
        let size = event.bounds.size.width.min(event.bounds.size.height) * DROP_TARGET_SIZE;

        // Reset the drag split direction when the mouse is no longer in this pane
        if !event.bounds.contains(&cursor_position) {
            self.drag_split_direction = None;
            cx.notify();
            return;
        }

        let direction = if relative_cursor.x < size
            || relative_cursor.x > pane_area.width - size
            || relative_cursor.y < size
            || relative_cursor.y > pane_area.height - size
        {
            [
                SplitDirection::Up,
                SplitDirection::Right,
                SplitDirection::Down,
                SplitDirection::Left,
            ]
            .iter()
            .min_by_key(|side| match side {
                SplitDirection::Up => relative_cursor.y,
                SplitDirection::Right => pane_area.width - relative_cursor.x,
                SplitDirection::Down => pane_area.height - relative_cursor.y,
                SplitDirection::Left => relative_cursor.x,
            })
            .cloned()
        } else {
            None
        };

        if direction != self.drag_split_direction {
            self.drag_split_direction = direction;
        }

        cx.notify();
    }

    // TODO:
    // - after splitted, dragging left to right's right should make the pane appears to the right
    // - active pane should turn to the focused editor's
    // - after closing all tabs, click sidebar items won't open new one
    fn handle_item_drop(
        &mut self,
        dragged_item: &DraggedItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(owner_pane) = dragged_item.owner_pane.clone() else {
            return;
        };

        let Some(owner_pane_id) = dragged_item.owner_pane_id else {
            return;
        };

        let Some(dragged_block_id) = dragged_item.block_id else {
            return;
        };

        let split_direction = self.drag_split_direction.take();
        let mut target_pane = cx.entity();
        let mut old_pane_has_opened_tabs = false;

        // This means the tab is dragged to the same pane but need to split
        if owner_pane_id == self.id {
            // Close the dragged tab to create the `move tab` effect
            self.close_tab(&dragged_block_id, cx);
            old_pane_has_opened_tabs = self.has_opened_blocks();
        }

        // Else, it means the tab is dragged to a different pane but need to split
        if owner_pane_id != self.id {
            dbg!("start updating owner pane!");
            old_pane_has_opened_tabs = owner_pane
                .update(cx, |this, cx| {
                    this.close_tab(&dragged_block_id, cx);
                    this.has_opened_blocks()
                })
                .unwrap();
            dbg!("owner pane updated!");

            if let Some(entity) = owner_pane.upgrade() {
                target_pane = entity;
            }
        }

        // Will recursively split
        // TODO: move the focus to the corresponding pane too
        let _ = self.pane_group.update(cx, |this, cx| {
            let pane_group_reference = cx.weak_entity();
            let new_pane = cx.new(|cx| Pane::new(cx, window, pane_group_reference));
            new_pane.update(cx, |this, cx| {
                let pane_id = this.id;
                this.set_selected_block_by_block_id(dragged_block_id, cx);
                cx.update_global::<States, ()>(|this, _cx| {
                    this.active_pane_id = Some(pane_id);
                });
            });

            if let Some(direction) = split_direction {
                this.split(&target_pane, &new_pane, direction, old_pane_has_opened_tabs);
            }

            cx.notify();
        });

        cx.notify();
    }

    pub fn set_selected_block_by_block_id(&mut self, block_id: Uuid, cx: &mut Context<Self>) {
        for opened_block_id in self.opened_block_ids.iter() {
            if *opened_block_id == block_id {
                self.selected_block_id = Some(*opened_block_id);
                cx.notify();
                return;
            }
        }

        self.opened_block_ids.push(block_id);
        self.selected_block_id = Some(block_id);
        cx.notify();
    }

    pub fn has_opened_blocks(&self) -> bool {
        !self.opened_block_ids.is_empty()
    }
}

impl Focusable for Pane {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Pane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_div = div().flex_1().flex_col(); // We need flex_1 to let the editor to take up the whole space after sidebar disappeared

        if self.opened_block_ids.is_empty() {
            return base_div.child("No documents yet");
        }

        let pane_reference = cx.weak_entity();
        let pane_id = self.id;

        let tabs = TabBar::new("tabs").children(self.opened_block_ids.iter().map(|id| {
            let id = id.clone();
            let mut selected = false;

            // The active block is the focused block
            if let Some(selected_block_id) = &self.selected_block_id {
                if *selected_block_id == id {
                    selected = true;
                }
            }

            let states: &States = cx.global();
            let mut title = String::new();
            if let Some(block) = states.blocks.get(&id) {
                title = block.get_title();
            }

            let dragged_item = DraggedItem {
                label: Some(SharedString::from(title.clone())),
                owner_pane: Some(pane_reference.clone()),
                owner_pane_id: Some(pane_id),
                block_id: Some(id),
                ..Default::default()
            };

            Tab::new()
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
                            if !view.has_opened_blocks() {
                                let pane = cx.entity();
                                let _ = view.pane_group.update(cx, |this, _cx| {
                                    this.remove_panes(&pane);
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
                )
            // .drag_over::<DraggedTab>(move |tab, dragged_tab: &DraggedTab, _, cx| {
            //     let styled_tab = tab
            //         .bg(cx.theme().blue)
            //         .border_color(cx.theme().blue)
            //         .border_0();

            //     styled_tab.border_r_2()
            // })
            // .on_drop(move |dragged: &DraggedTab, window, app| {

            // })
        }));

        // Open editor only when there is an active block
        if self.selected_block_id.is_none() {
            return base_div.child(tabs);
        };

        let editor = self.editor.clone();
        editor.update(cx, |this, cx| {
            // The backend is always the source of truth.
            // We fetch the block from the backend with the current uuid.

            if let Some(selected_block_id) = self.selected_block_id {
                let states: &States = cx.global();
                if let Some(block) = states.blocks.get(&selected_block_id) {
                    this.register_block(block.clone());
                }
            }
        });

        log::debug!("Rendering the pane... {:?}", self.drag_split_direction);
        base_div.h_full().child(tabs).child(
            div()
                .h_full()
                .on_drag_move::<DraggedItem>(cx.listener(Self::handle_drag_move)) // Calculate the split preview area
                .child(editor)
                .child(
                    div()
                        .absolute()
                        .bg(cx.theme().blue.opacity(0.3)) // Split preview style
                        .invisible()
                        .on_drop(cx.listener(
                            move |this, dragged_item: &DraggedItem, window, cx| {
                                this.handle_item_drop(dragged_item, window, cx);
                            },
                        ))
                        .map(|div| {
                            // Note that we didn't use group drag over like Zed,
                            // because it didn't work here after several tries.
                            // Might need to look into this further if there are further issues.
                            log::debug!("Render split previews...");
                            let size = DefiniteLength::Fraction(0.5);
                            match self.drag_split_direction {
                                None => div.top_0().right_0().bottom_0().left_0(),
                                Some(SplitDirection::Up) => {
                                    div.top_0().left_0().right_0().h(size).visible() // Set to visible when dragged over this area
                                }
                                Some(SplitDirection::Down) => {
                                    div.left_0().bottom_0().right_0().h(size).visible()
                                }
                                Some(SplitDirection::Left) => {
                                    div.top_0().left_0().bottom_0().w(size).visible()
                                }
                                Some(SplitDirection::Right) => {
                                    div.top_0().bottom_0().right_0().w(size).visible()
                                }
                            }
                        }),
                ),
        )
    }
}
