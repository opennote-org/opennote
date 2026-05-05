use gpui::{
    AppContext, BorrowAppContext, Entity, InteractiveElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, prelude::FluentBuilder, px,
};
use gpui_component::{IconName, h_flex, list::ListItem, menu::ContextMenuExt};
use uuid::Uuid;

use crate::{
    globals::{
        actions::{delete_n_blocks, update_parent},
        states::States,
    },
    key_mappings::mappings::{CreateOneBlock, DeleteBlocks},
    libs::tree::drag::DraggedBlocks,
    widgets::sidebar::OpenNoteSidebar,
};

// This is for creating the root block that allows other blocks to be able to
// drag back to root
pub fn create_root_tree_list_item(
    index: usize,
    entry: &crate::libs::tree::TreeEntry,
    id: SharedString, // The id of the tree item
    uuid: Uuid,
    sidebar_entity_on_drop: Entity<OpenNoteSidebar>,
    sidebar_entity_on_drag_move: Entity<OpenNoteSidebar>,
    is_dragged_over: bool,
) -> ListItem {
    ListItem::new(index)
        .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
        .check_icon(IconName::Check)
        .cursor_move()
        .child(
            h_flex()
                .id(id.clone())
                .gap_2()
                .child("--------------------------")
                .when(is_dragged_over, |this| {
                    this.border_b_2().border_color(gpui::blue())
                })
                .on_drag_move::<DraggedBlocks>(move |event, window, app| {
                    sidebar_entity_on_drag_move.update(app, |this, cx| {
                        // Update the dragged block when the mouse moves into a bound of list item
                        if event.bounds.contains(&event.event.position) {
                            this.tree_state.update(cx, |this, cx| {
                                this.dragged_target_block = Some(uuid);
                            });
                            cx.notify();
                        }
                    });
                })
                .on_drop(move |dragged: &DraggedBlocks, window, app| {
                    sidebar_entity_on_drop.update(app, |this, cx| {
                        this.tree_state.update(cx, |this, cx| {
                            this.dragged_target_block = None;

                            this.selected_block = None;
                            this.selected_blocks.clear();
                        });

                        cx.update_global::<States, ()>(|global, cx| {
                            update_parent(cx, None, dragged.current_selections.clone());
                        });

                        cx.notify();
                    });
                }),
        )
}

pub fn create_tree_list_item(
    index: usize,
    entry: &crate::libs::tree::TreeEntry,
    label: SharedString, // The label of the tree item. Usually is the title of a block
    id: SharedString,    // The id of the tree item
    uuid: Uuid,          // The uuid/id of the block
    language_profile: crate::globals::assets::LanguageProfile,
    sidebar_entity_delete_blocks: Entity<OpenNoteSidebar>,
    sidebar_entity_on_mouse_down: Entity<OpenNoteSidebar>,
    sidebar_entity_on_drop: Entity<OpenNoteSidebar>,
    sidebar_entity_on_drag_move: Entity<OpenNoteSidebar>,
    is_selected: bool,
    is_multi_selected: bool,
    is_dragged_over: bool,
    dragged_block: DraggedBlocks,
) -> ListItem {
    ListItem::new(index)
        .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
        .check_icon(IconName::Check)
        .when(is_selected || is_multi_selected, |this| this.selected(true))
        .cursor_move()
        .child(
            h_flex()
                .id(id.clone())
                .gap_2()
                .when(is_dragged_over, |this| {
                    this.border_b_2().border_color(gpui::blue())
                })
                .child(label)
                .on_drag(dragged_block.clone(), |value, _point, _window, app| {
                    app.new(|_| value.clone())
                })
                .on_drop(move |dragged: &DraggedBlocks, _window, app| {
                    sidebar_entity_on_drop.update(app, |this, cx| {
                        if dragged.block_id == uuid {
                            return;
                        }

                        this.tree_state.update(cx, |this, _cx| {
                            this.dragged_target_block = None;
                            this.selected_block = None;
                            this.selected_blocks.clear();
                        });

                        cx.update_global::<States, ()>(|global, cx| {
                            update_parent(cx, Some(uuid), dragged.current_selections.clone());
                        });

                        cx.notify();
                    });
                })
                .on_drag_move::<DraggedBlocks>(move |event, window, app| {
                    sidebar_entity_on_drag_move.update(app, |this, cx| {
                        // Update the dragged block when the mouse moves into a bound of list item
                        if event.bounds.contains(&event.event.position) {
                            this.tree_state.update(cx, |this, cx| {
                                this.dragged_target_block = Some(uuid);
                            });
                            cx.notify();
                        }
                    });
                })
                .on_action(move |_action: &DeleteBlocks, _window, cx| {
                    sidebar_entity_delete_blocks.update(cx, |this, cx| {
                        let mut to_delete = Vec::new();

                        this.tree_state.update(cx, |this, cx| {
                            let is_multi_selected = !this.selected_blocks.is_empty();

                            if is_multi_selected {
                                to_delete.extend(this.selected_blocks.to_owned());
                                this.selected_blocks.clear();
                            }

                            if !is_multi_selected {
                                if let Some(block) = this.selected_block.take() {
                                    to_delete.push(block);
                                }
                            }
                        });

                        log::debug!("About to delete blocks: {:?}", to_delete);
                        delete_n_blocks(cx, to_delete);
                        cx.notify();
                    });
                })
                .on_click(move |event, _window, app| {
                    sidebar_entity_on_mouse_down.update(app, |this, cx| {
                        let id = OpenNoteSidebar::convert_str_to_uuid(&id.clone()).unwrap();

                        // Multi-selection only happens when the platform key is pressed
                        if event.modifiers().platform {
                            this.tree_state.update(cx, |this, cx| {
                                // Single selection should be converted to multi-selection
                                if let Some(selected) = this.selected_block {
                                    let has_single_selected = selected == id;
                                    this.selected_block = None;

                                    // Multi-selecting a single selected item will deselect the item
                                    if has_single_selected {
                                        return;
                                    }
                                }

                                // Each selection must be unique
                                if !this.selected_blocks.insert(id) {
                                    // Deselect the already multi-selected
                                    this.selected_blocks.remove(&id);
                                }
                            });
                        }

                        if !event.modifiers().platform {
                            this.tree_state.update(cx, |this, cx| {
                                this.selected_blocks.clear();
                                this.selected_block = Some(id)
                            });
                        }

                        cx.notify();
                    });
                })
                .context_menu(move |menu, _window, _cx| {
                    menu.menu(
                        language_profile.create_one_block.clone(),
                        Box::new(CreateOneBlock),
                    )
                    .menu(
                        language_profile.delete_blocks.clone(),
                        Box::new(DeleteBlocks),
                    )
                }),
        )
}
