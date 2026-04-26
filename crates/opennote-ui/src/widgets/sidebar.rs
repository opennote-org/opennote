use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use gpui::{
    AppContext, BorrowAppContext, Context, Entity, EntityId, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Side, button::Button, h_flex, label::Label, list::ListItem, menu::ContextMenuExt,
};
use uuid::Uuid;

use crate::{
    globals::{
        actions::{create_one_block, delete_n_blocks, update_parent},
        helpers::get_language_profile,
        states::{ProtectedBlock, States},
    },
    key_mappings::{
        key_contexts::SIDEBAR,
        mappings::{CreateOneBlock, DeleteBlocks},
    },
    libs::{
        tree::{Tree, TreeState, tree},
        tree_view_sidebar::TreeViewSidebar,
    },
    widgets::blocks_tree::build_blocks_tree,
};

#[derive(Debug, Clone)]
pub struct DraggedBlocks {
    pub block_id: Uuid,
    pub label: SharedString,
    pub current_selections: Vec<Uuid>,
}

impl Render for DraggedBlocks {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .shadow_md()
            .bg(gpui::white())
            .opacity(0.85)
            .text_sm()
            .child(format!("{} items", self.current_selections.len()))
    }
}

pub struct OpenNoteSidebar {
    focus_handle: FocusHandle,
    is_toggled: bool,
    tree_state: Entity<TreeState>,
    selected_block: Option<Uuid>,
    selected_blocks: HashSet<Uuid>,
    dragged_target_block: Option<Uuid>,

    _subscriptions: Vec<Subscription>,
}

impl OpenNoteSidebar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut _subscriptions = Vec::new();

        let tree_state = cx.new(|cx| TreeState::new(cx));

        // Watch for changes in States, such as the blocks list
        _subscriptions.push(cx.observe_global::<States>(|_this, cx| {
            log::debug!("Sidebar refreshes because the global state had changed");
            cx.notify();
        }));

        _subscriptions.push(cx.observe(&tree_state, |this, _tree_state, cx| {
            let Some(uuid) = this.selected_block else {
                return;
            };

            cx.update_global::<States, ()>(|global, _cx| {
                let selected_block = {
                    let blocks = global.blocks.read().unwrap();
                    let mut selected_block: Vec<&ProtectedBlock> = blocks
                        .iter()
                        .filter(|item| item.0.read().unwrap().id == uuid)
                        .collect();
                    selected_block.remove(0).clone()
                };

                global.set_active_block(selected_block.clone());
                log::debug!(
                    "Set active block to {}",
                    selected_block.0.read().unwrap().id
                );
            });
        }));

        Self {
            focus_handle: cx.focus_handle(), // obtain a new focus from the global pool for this view
            is_toggled: true,
            tree_state,
            dragged_target_block: None,
            selected_block: None,
            selected_blocks: HashSet::new(),
            _subscriptions,
        }
    }

    pub fn is_toggled(&self) -> bool {
        self.is_toggled
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.is_toggled = !self.is_toggled;
        cx.notify();
    }

    /// Use .unwrap by default. Make sure the input is a valid uuid string
    fn convert_str_to_uuid(str: &str) -> Result<Uuid> {
        Ok(Uuid::parse_str(str)?)
    }

    /// Determine whether the sidebar item is single selected.
    fn is_sidebar_item_single_selected(&self, sidebar_item_id: Uuid) -> bool {
        let mut is_single_selected = false;

        // Check against the single selection
        if let Some(block) = self.selected_block {
            if block == sidebar_item_id {
                is_single_selected = true;
            }
        }

        is_single_selected
    }

    /// Determine whether the sidebar item is multi selected
    fn is_sidebar_item_multi_selected(&self, sidebar_item_id: Uuid) -> bool {
        let mut is_confirmed = false;

        // Check against the multi-selection
        if self.selected_blocks.contains(&sidebar_item_id) {
            is_confirmed = true;
        }

        is_confirmed
    }

    fn create_sidebar_items(
        &self,
        cx: &mut Context<Self>,
        blocks: Arc<RwLock<Vec<ProtectedBlock>>>,
    ) -> Tree {
        log::debug!("Building sidebar items...");
        let tree_items = build_blocks_tree(blocks);

        self.tree_state.update(cx, |this, cx| {
            this.set_items(tree_items, cx);
        });

        // We need this to update the sidebar's internal state
        let sidebar = cx.entity();

        let tree = tree(&self.tree_state, move |index, entry, _window, cx| {
            let id = entry.item().id.clone(); // This is a stringified uuid of a block
            let label = entry.item().label.clone();
            let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
            let sidebar_entity_delete_blocks = sidebar.clone();
            let sidebar_entity_on_mouse_down = sidebar.clone();
            let sidebar_entity_on_drop = sidebar.clone();
            let sidebar_entity_on_drag_move = sidebar.clone();

            let uuid = Self::convert_str_to_uuid(&id).unwrap();

            let is_dragged_over = sidebar.read(cx).dragged_target_block == Some(uuid);

            // Create a root tree list item for being able to drag blocks
            // back to the root
            if label == "root" {
                return create_root_tree_list_item(
                    index,
                    entry,
                    id,
                    uuid,
                    sidebar_entity_on_drop,
                    sidebar_entity_on_drag_move,
                    is_dragged_over,
                );
            }

            let is_selected = sidebar.read(cx).is_sidebar_item_single_selected(uuid);
            let is_multi_selected = sidebar.read(cx).is_sidebar_item_multi_selected(uuid);

            let current_selections = if let Some(dragged) = sidebar.read(cx).selected_block {
                vec![dragged]
            } else {
                sidebar.read(cx).selected_blocks.clone().drain().collect()
            };

            let dragged_block = DraggedBlocks {
                block_id: uuid,
                label: label.clone(),
                current_selections,
            };

            create_tree_list_item(
                index,
                entry,
                label,
                id,
                uuid,
                language_profile,
                sidebar_entity_delete_blocks,
                sidebar_entity_on_mouse_down,
                sidebar_entity_on_drop,
                sidebar_entity_on_drag_move,
                is_selected,
                is_multi_selected,
                is_dragged_over,
                dragged_block,
            )
        });

        tree
    }

    fn create_new_block_button(entity_id: EntityId) -> Button {
        Button::new("workspace_sidebar_create_new_block_button")
            .label("+")
            .on_click(move |click, _window, app_cx| {
                if !click.is_right_click() {
                    // Default to create a root block
                    create_one_block(app_cx, None);
                    app_cx.notify(entity_id);
                }
            })
    }
}

// This is for creating the root block that allows other blocks to be able to
// drag back to root
fn create_root_tree_list_item(
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
                            this.dragged_target_block = Some(uuid);
                            cx.notify();
                        }
                    });
                })
                .on_drop(move |dragged: &DraggedBlocks, window, app| {
                    sidebar_entity_on_drop.update(app, |this, cx| {
                        this.dragged_target_block = None;

                        this.selected_block = None;
                        this.selected_blocks.clear();

                        cx.update_global::<States, ()>(|global, cx| {
                            update_parent(cx, None, dragged.current_selections.clone());
                        });

                        cx.notify();
                    });
                }),
        )
}

fn create_tree_list_item(
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
                        this.dragged_target_block = None;

                        if dragged.block_id == uuid {
                            return;
                        }

                        this.selected_block = None;
                        this.selected_blocks.clear();

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
                            this.dragged_target_block = Some(uuid);
                            cx.notify();
                        }
                    });
                })
                .on_action(move |_action: &DeleteBlocks, _window, cx| {
                    sidebar_entity_delete_blocks.update(cx, |this, cx| {
                        let mut to_delete = Vec::new();
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
                        }

                        if !event.modifiers().platform {
                            this.selected_blocks.clear();
                            this.selected_block = Some(id)
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

impl Focusable for OpenNoteSidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OpenNoteSidebar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
        let states: &States = cx.global();
        let entity_id = cx.entity_id();

        log::debug!("Refreshing sidebar...");
        log::debug!("Single selected block: {:?}", self.selected_block);
        log::debug!("Multi selected blocks: {:?}", self.selected_blocks);
        log::debug!("Got {} blocks", states.blocks.read().unwrap().len());

        div()
            .key_context(SIDEBAR)
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .when(self.is_toggled, |this| this.visible())
            .when(!self.is_toggled, |this| this.invisible())
            .child(
                TreeViewSidebar::new(Side::Left)
                    .child(self.create_sidebar_items(cx, states.blocks.clone()))
                    .header(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(Label::new(language_profile.sidebar_title).text_xl())
                            .child(Self::create_new_block_button(entity_id)),
                    ),
            )
            .on_action(cx.listener(|this, _action: &CreateOneBlock, _window, cx| {
                let mut parent_block_id = None;
                if let Some(block) = this.selected_block {
                    parent_block_id = Some(block)
                }

                log::debug!("About to create a block under: {:?}", parent_block_id);
                create_one_block(cx, parent_block_id);
                cx.notify();
            }))
    }
}
