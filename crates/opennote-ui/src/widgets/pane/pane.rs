use gpui::{Action, DragMoveEvent, ElementId, Entity, Point, SharedString, Subscription, div};
use gpui::{App, Context, FocusHandle, Focusable, Render, Window, prelude::*};
use gpui_component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_component::{IconName, Selectable, Sizable};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use opennote_models::block::Block;

use crate::globals::states::States;
use crate::libs::tabs::drag::DraggedItem;
use crate::libs::tabs::tab::Tab;
use crate::libs::tabs::tab_bar::TabBar;
use crate::widgets::editor::Editor;
use crate::widgets::pane::pane_group::SplitDirection;

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

const MAX_NAVIGATION_HISTORY_LEN: usize = 1024;
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
    opened_blocks: Vec<Block>,
    editor: Entity<Editor>,
    drag_split_direction: Option<SplitDirection>,
    // focus_handle: FocusHandle,
    // active_item_index: usize,
    // pub drag_split_direction: Option<SplitDirection>,
    _subscriptions: Vec<Subscription>,
}

impl Pane {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        let mut _subscriptions = Vec::new();

        // However, we can use observe_global_in to use `window`, if needed
        _subscriptions.push(cx.observe_global::<States>(|this, cx| {
            log::debug!("Editor refreshes because the global state had changed");

            cx.read_global::<States, ()>(|states, _cx| {
                // The active block sticks with the global active block
                if let Some(active_block_id) = &states.active_block_id {
                    // Skip if the block has been openned
                    for opened_block in this.opened_blocks.iter() {
                        if opened_block.id == *active_block_id {
                            return;
                        }
                    }

                    let block = states.get_active_block();
                    if let Some(block) = block {
                        this.opened_blocks.push(block.clone());
                    }
                }
            });

            cx.notify();
        }));

        Self {
            drag_split_direction: None,
            editor: cx.new(|cx| Editor::new(cx, window)),
            opened_blocks: Vec::new(),
            _subscriptions,
        }
    }

    fn close_tab(&mut self, block_id: Uuid, cx: &mut Context<Self>) {
        let states: &States = cx.global();
        let active_block = states.get_active_block();

        // if we have multiple tabs openning
        if self.opened_blocks.len() > 1 {
            // Remove the closed block from the openned blocks,
            // while also retain an index for moving the focus to the prevoius one
            let mut removed_index: isize = 0;
            for (index, block) in self.opened_blocks.iter().enumerate() {
                if block.id == block_id && index != 0 {
                    removed_index = index as isize;
                    break;
                }
            }

            self.opened_blocks.remove(removed_index as usize);

            // Move the focus to the previous tab / block
            if let Some(active_block) = active_block {
                let mut index_to_focus = removed_index - 1;

                // Handle if the closed tab is the first one with no previous tabs
                if index_to_focus < 0 {
                    index_to_focus = 0;
                }

                let Some(move_to_block) = self.opened_blocks.get(index_to_focus as usize) else {
                    return;
                };

                // Move the focus only when the active block has been closed
                if active_block.id == block_id {
                    cx.update_global::<States, ()>(|this, _cx| {
                        this.set_active_block_id(move_to_block.id);
                    });
                }
            }

            cx.notify();

            // Prevent triggering the 1 tab case when
            // the openned tabs become 1 after the tab closing
            return;
        }

        // if we only have 1 tab openning
        if self.opened_blocks.len() == 1 {
            self.opened_blocks.clear();
            cx.update_global::<States, ()>(|this, _cx| {
                this.active_block_id = None;
            });

            cx.notify();
        }

        // no tab closing for 0 tabs
    }

    fn handle_drag_move<T: 'static>(
        &mut self,
        event: &DragMoveEvent<T>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let pane_area = event.bounds.size;

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
    }
}

// impl Focusable for Pane {
//     fn focus_handle(&self, _cx: &App) -> FocusHandle {
//         self.focus_handle.clone()
//     }
// }

impl Render for Pane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_div = div().flex_1().flex_col(); // We need flex_1 to let the editor to take up the whole space after sidebar disappeared

        if self.opened_blocks.is_empty() {
            return base_div.child("No documents yet");
        }

        let tabs = TabBar::new("tabs").children(self.opened_blocks.iter().map(|item| {
            let id = item.id;
            let mut selected = false;

            let states: &States = cx.global();
            // The active block is the focused block
            if let Some(active_block_id) = states.active_block_id {
                if active_block_id == id {
                    selected = true;
                }
            }

            Tab::new()
                .label(item.get_title())
                .selected(selected)
                .suffix(
                    Button::new(ElementId::Name(SharedString::from(format!("close-{}", id))))
                        .icon(IconName::CircleX)
                        .ghost()
                        .xsmall()
                        .rounded(ButtonRounded::Medium)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.close_tab(id, cx);
                            cx.stop_propagation();
                        })),
                )
                .on_click(move |event, _window, cx| {
                    if !event.is_right_click() {
                        cx.update_global(|this: &mut States, _cx| {
                            this.set_active_block_id(id);
                        });
                    }
                })
        }));

        // Open editor only when there is an active block
        let states: &States = cx.global();
        let Some(_) = states.active_block_id.clone() else {
            return base_div.child(tabs);
        };

        let editor = self.editor.clone();
        editor.update(cx, |this, cx| {
            // The backend is always the source of truth.
            // We fetch the block from the backend with the current uuid.

            let states: &States = cx.global();

            let block = states.get_active_block();

            if let Some(block) = block {
                this.register_block(block.clone());
            }
        });

        base_div
            .h_full()
            .child(tabs)
            .child(editor)
            .on_drag_move::<DraggedItem>(cx.listener(Self::handle_drag_move)) // TODO: render the split preview
    }
}
