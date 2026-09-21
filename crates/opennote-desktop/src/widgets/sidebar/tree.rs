use std::collections::HashMap;

use gpui_kit::component::{
    ActiveTheme, IconName, InteractiveElementExt, Sizable,
    button::{Button, ButtonRounded, ButtonVariants},
    h_flex,
    list::ListItem,
    menu::ContextMenuExt,
};
use gpui_kit::{
    App, AppContext, BorrowAppContext, ClickEvent, ElementId, Entity, InteractiveElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
    px,
};
use uuid::Uuid;

use crate::{
    globals::{
        actions::{delete_n_blocks, update_parent},
        states::States,
    },
    key_mappings::mappings::{CreateOneBlock, DeleteBlocks, ExportFiles, ImportFiles},
    libs::{tabs::drag::DraggedItem, tree::TreeState},
    widgets::sidebar::{OpenNoteSidebar, OpenNoteSidebarEvent},
};

// Collect blocks to drag from both the single selection and the multi-selection.
fn collect_selections(dragged: &DraggedItem) -> Vec<Uuid> {
    let mut blocks_to_drag = vec![];

    // Collect from single selection
    if let Some(dragged_block) = dragged.block_id {
        blocks_to_drag.push(dragged_block);
    }

    // Collection from multi-selection
    blocks_to_drag.extend(dragged.selections.clone());

    blocks_to_drag
}

fn has_mouse_moved(event: &ClickEvent, this: &mut OpenNoteSidebar) -> bool {
    // Determine if the mouse has been dragged or clicked
    if let Some(position) = event.mouse_position() {
        let Some(mouse_position) = this.mouse_position.take() else {
            return false;
        };

        // If the mouse has not moved, we continue to on click
        if position != mouse_position {
            return true;
        }
    }

    false
}

// This is for creating the root block that allows other blocks to be able to
// drag back to root
pub fn create_root_tree_list_item(
    index: usize,
    entry: &crate::libs::tree::TreeEntry,
    id: SharedString, // The id of the tree item
    uuid: Uuid,
    tree_state: Entity<TreeState>,
    sidebar: Entity<OpenNoteSidebar>,
    is_dragged_over: bool,
) -> ListItem {
    let sidebar_entity_on_drop: Entity<OpenNoteSidebar> = sidebar.clone();
    let sidebar_entity_on_drag_move: Entity<OpenNoteSidebar> = sidebar.clone();

    let tree_state_on_drag_move = tree_state.clone();

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
                    this.border_b_2().border_color(gpui_kit::blue())
                })
                .on_drag_move::<DraggedItem>(move |event, _window, app| {
                    sidebar_entity_on_drag_move.update(app, |_, cx| {
                        // Update the dragged block when the mouse moves into a bound of list item
                        if event.bounds.contains(&event.event.position) {
                            tree_state.update(cx, |this, _cx| {
                                this.dragged_target_block = Some(uuid);
                            });
                            cx.notify();
                        }
                    });
                })
                .on_drop(move |dragged: &DraggedItem, window, app| {
                    sidebar_entity_on_drop.update(app, |this, cx| {
                        this.mouse_position = None;

                        let blocks_to_drag: Vec<Uuid> = collect_selections(dragged);

                        tree_state_on_drag_move.update(cx, |this, _cx| {
                            this.dragged_target_block = None;

                            this.selected_block = None;
                            this.selected_blocks.clear();
                        });

                        cx.update_global::<States, ()>(|_global, cx| {
                            update_parent(window, cx, None, blocks_to_drag);
                        });

                        cx.notify();
                    });
                }),
        )
}

pub fn create_tree_list_item(
    cx: &mut App,
    index: usize,
    entry: &crate::libs::tree::TreeEntry,
    label: SharedString, // The label of the tree item. Usually is the title of a block
    id: SharedString,    // The id of the tree item
    uuid: Uuid,          // The uuid/id of the block
    language_profile: HashMap<String, String>,
    sidebar: Entity<OpenNoteSidebar>,
    tree_state: Entity<TreeState>,
    is_selected: bool,
    is_multi_selected: bool,
    is_dragged_over: bool,
    dragged_block: DraggedItem,
    has_children: bool,
) -> ListItem {
    let sidebar_entity_delete_blocks: Entity<OpenNoteSidebar> = sidebar.clone();
    let sidebar_entity_on_drop: Entity<OpenNoteSidebar> = sidebar.clone();
    let sidebar_entity_on_drag_move: Entity<OpenNoteSidebar> = sidebar.clone();
    let sidebar_entity_on_mouse_click: Entity<OpenNoteSidebar> = sidebar.clone();
    let sidebar_entity_on_mouse_right_click: Entity<OpenNoteSidebar> = sidebar.clone();
    let sidebar_entity_expand: Entity<OpenNoteSidebar> = sidebar.clone();

    let tree_state_entity_delete_blocks = tree_state.clone();
    let tree_state_entity_on_drop = tree_state.clone();
    let tree_state_entity_on_drag_move = tree_state.clone();
    let tree_state_entity_on_mouse_click = tree_state.clone();
    let tree_state_entity_on_mouse_right_click = tree_state.clone();

    let block_state = sidebar_entity_expand
        .read(cx)
        .block_states
        .get_block_state(uuid);

    let has_expanded = match block_state {
        Some(result) => result.has_expanded,
        None => false,
    };

    let depth = entry.depth();
    let indent_guide_color = cx.theme().sidebar_border;

    ListItem::new(index)
        .w_full() // Let the background highlights take over the entire row for the short ones as well
        .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
        .when(is_selected || is_multi_selected, |this| this.selected(true))
        .cursor_move()
        .relative()
        .suffix(move |window, cx| create_indent_guide(window, cx, depth, indent_guide_color))
        .child(
            h_flex()
                .when(has_children, |this| {
                    render_parent_button(
                        has_expanded,
                        index,
                        &id,
                        uuid,
                        &tree_state,
                        sidebar_entity_expand,
                        this,
                    )
                })
                .when(!has_children, |this| {
                    render_non_parent_button(has_expanded, &id, this)
                })
                .id(id.clone())
                .gap_2()
                .when(is_dragged_over, |this| {
                    this.border_b_2().border_color(gpui_kit::blue())
                })
                .child(label)
                .on_mouse_down(gpui_kit::MouseButton::Left, move |event, _window, cx| {
                    start_mouse_dragging(&sidebar, event, cx);
                })
                .on_mouse_down(gpui_kit::MouseButton::Right, move |_event, _window, cx| {
                    handle_sidebar_item_right_click(
                        uuid,
                        &tree_state_entity_on_mouse_right_click,
                        &sidebar_entity_on_mouse_right_click,
                        cx,
                    )
                })
                .on_drag(dragged_block.clone(), |value, _point, _window, app| {
                    app.new(|_| value.clone())
                })
                .on_drop(handle_sidebar_items_drop(
                    uuid,
                    tree_state_entity_on_drop,
                    sidebar_entity_on_drop,
                ))
                .on_drag_move::<DraggedItem>(handle_sidebar_items_move(
                    uuid,
                    tree_state_entity_on_drag_move,
                    sidebar_entity_on_drag_move,
                ))
                .on_action(handle_sidebar_delete_item(
                    tree_state_entity_delete_blocks,
                    sidebar_entity_delete_blocks,
                ))
                .on_double_click(handle_sidebar_item_double_click(
                    uuid,
                    tree_state_entity_on_mouse_click.clone(),
                    sidebar_entity_on_mouse_click.clone(),
                ))
                .on_click(handle_sidebar_item_click(
                    uuid,
                    tree_state_entity_on_mouse_click,
                    sidebar_entity_on_mouse_click,
                ))
                .context_menu(move |menu, _window, _cx| {
                    menu.menu(
                        &language_profile["create_one_block"],
                        Box::new(CreateOneBlock),
                    )
                    .menu(&language_profile["delete_blocks"], Box::new(DeleteBlocks))
                    .menu(
                        &language_profile["workspace::ImportFiles"],
                        Box::new(ImportFiles),
                    )
                    .menu(
                        &language_profile["workspace::ExportFiles"],
                        Box::new(ExportFiles),
                    )
                }),
        )
}

fn handle_sidebar_item_double_click(
    uuid: Uuid,
    tree_state: Entity<TreeState>,
    sidebar_entity_on_mouse_click: Entity<OpenNoteSidebar>,
) -> impl Fn(&ClickEvent, &mut gpui_kit::Window, &mut App) {
    move |_event, window, app| {
        sidebar_entity_on_mouse_click.update(app, |this, cx| {
            // Reset the mouse position
            this.mouse_position = None;
            this.handle_block_open(uuid, cx, window, tree_state.clone());
        });
    }
}

fn handle_sidebar_item_right_click(
    uuid: Uuid,
    tree_state: &Entity<TreeState>,
    sidebar: &Entity<OpenNoteSidebar>,
    cx: &mut App,
) {
    sidebar.update(cx, |this, cx| {
        // Reset the mouse position
        this.mouse_position = None;

        tree_state.update(cx, |this, cx| {
            let has_multi_selected = !this.selected_blocks.is_empty();

            // Prevent the right click canceling multi-selections
            if has_multi_selected {
                return;
            }

            this.selected_blocks.clear();
            this.selected_block = Some(uuid);

            cx.notify();
        });

        cx.notify();
        return;
    });
}

fn handle_sidebar_item_click(
    uuid: Uuid,
    tree_state: Entity<TreeState>,
    sidebar_entity_on_mouse_click: Entity<OpenNoteSidebar>,
) -> impl Fn(&ClickEvent, &mut gpui_kit::Window, &mut App) {
    move |event, _window, app| {
        sidebar_entity_on_mouse_click.update(app, |this, cx| {
            if has_mouse_moved(event, this) {
                // Because this means a drag, not a click
                return;
            }

            // Reset the mouse position
            this.mouse_position = None;

            // Multi-selection only happens when the platform key is pressed,
            // and is using the left click
            if event.modifiers().platform && !event.is_right_click() {
                tree_state.update(cx, |this, _cx| {
                    // Single selection should be converted to multi-selection
                    if let Some(selected) = this.selected_block {
                        let has_single_selected = selected == uuid;
                        this.selected_block = None;

                        // Multi-selecting a single selected item will deselect the item
                        if has_single_selected {
                            return;
                        }
                    }

                    // Each selection must be unique
                    if !this.selected_blocks.insert(uuid) {
                        // Deselect the already multi-selected
                        this.selected_blocks.remove(&uuid);
                    }
                });
            }

            cx.notify();
            return;
        });
    }
}

fn handle_sidebar_delete_item(
    tree_state: Entity<TreeState>,
    sidebar_entity_delete_blocks: Entity<OpenNoteSidebar>,
) -> impl Fn(&DeleteBlocks, &mut gpui_kit::Window, &mut App) {
    move |_action: &DeleteBlocks, window, cx| {
        sidebar_entity_delete_blocks.update(cx, |_this, cx| {
            let mut to_delete = Vec::new();

            tree_state.update(cx, |this, _cx| {
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

            delete_n_blocks(window, cx, to_delete.clone());

            cx.emit(OpenNoteSidebarEvent::BlocksDeleted(to_delete));

            cx.notify();
        });
    }
}

fn handle_sidebar_items_move(
    uuid: Uuid,
    tree_state: Entity<TreeState>,
    sidebar_entity_on_drag_move: Entity<OpenNoteSidebar>,
) -> impl Fn(&gpui_kit::DragMoveEvent<DraggedItem>, &mut gpui_kit::Window, &mut App) {
    move |event, _window, app| {
        sidebar_entity_on_drag_move.update(app, |_this, cx| {
            // Update the dragged block when the mouse moves into a bound of list item
            if event.bounds.contains(&event.event.position) {
                tree_state.update(cx, |this, _cx| {
                    this.dragged_target_block = Some(uuid);
                });
                cx.notify();
            }
        });
    }
}

fn handle_sidebar_items_drop(
    uuid: Uuid,
    tree_state: Entity<TreeState>,
    sidebar_entity_on_drop: Entity<OpenNoteSidebar>,
) -> impl Fn(&DraggedItem, &mut gpui_kit::Window, &mut App) {
    move |dragged: &DraggedItem, window, app| {
        sidebar_entity_on_drop.update(app, |this, cx| {
            this.mouse_position = None;

            if dragged.block_id == Some(uuid) {
                return;
            }

            let blocks_to_drag: Vec<Uuid> = collect_selections(dragged);

            tree_state.update(cx, |this, _cx| {
                this.dragged_target_block = None;
                this.selected_block = None;
                this.selected_blocks.clear();
            });

            cx.update_global::<States, ()>(|_global, cx| {
                update_parent(window, cx, Some(uuid), blocks_to_drag);
            });

            cx.notify();
        });
    }
}

fn start_mouse_dragging(
    sidebar_entity_on_mouse_down: &Entity<OpenNoteSidebar>,
    event: &gpui_kit::MouseDownEvent,
    cx: &mut App,
) {
    // This is to prevent the dragging operations being covered up by on clicks.
    // We use the mouse position to determine if the item is dragged or clicked.
    sidebar_entity_on_mouse_down.update(cx, |this, _cx| {
        this.mouse_position = Some(event.position);
    });
}

fn render_non_parent_button(
    has_expanded: bool,
    id: &SharedString,
    this: gpui_kit::Div,
) -> gpui_kit::Div {
    let icon = match has_expanded {
        true => include_bytes!("../../../assets/icons/file-open.svg").as_slice(),
        false => include_bytes!("../../../assets/icons/file-close.svg").as_slice(),
    };

    this.child(
        Button::new(ElementId::Name(SharedString::from(format!(
            "expand-{}",
            id
        ))))
        .icon(gpui_kit::component::Icon::default().data(icon))
        .ghost()
        .xsmall()
        .rounded(ButtonRounded::Medium),
    )
}

fn render_parent_button(
    has_expanded: bool,
    index: usize,
    id: &SharedString,
    uuid: Uuid,
    tree_state: &Entity<TreeState>,
    sidebar_entity_expand: Entity<OpenNoteSidebar>,
    this: gpui_kit::Div,
) -> gpui_kit::Div {
    let tree_state = tree_state.clone();

    let icon = match has_expanded {
        true => IconName::FolderOpen,
        false => IconName::FolderClosed,
    };

    this.child(
        Button::new(ElementId::Name(SharedString::from(format!(
            "expand-{}",
            id
        ))))
        .icon(icon)
        .ghost()
        .xsmall()
        .rounded(ButtonRounded::Medium)
        .on_click(move |event, window, cx| {
            if !event.is_right_click() {
                sidebar_entity_expand.update(cx, |this, cx| {
                    tree_state.update(cx, |this, cx| {
                        this.on_entry_click(index, window, cx);
                    });

                    this.block_states.toggle_expansion(uuid);

                    cx.notify();
                })
            }

            cx.stop_propagation();
        }),
    )
}

fn create_indent_guide(
    _: &mut gpui_kit::Window,
    _: &mut App,
    depth: usize,
    indent_guide_color: gpui_kit::Hsla,
) -> gpui_kit::Div {
    div()
        .absolute()
        .left_0()
        .top_0()
        .bottom_0()
        .w(px(0.))
        .children((0..depth).map(move |level| {
            div()
                .absolute()
                .left(px(20. + 16. * level as f32))
                .top_0()
                .bottom_0()
                .w(px(1.))
                .bg(indent_guide_color)
        }))
}
