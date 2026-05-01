pub mod tree;

use std::sync::{Arc, RwLock};

use anyhow::Result;
use gpui::{
    AppContext, BorrowAppContext, Context, Entity, EntityId, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, Styled, Subscription, div,
};
use gpui_component::{Side, button::Button, h_flex, label::Label};
use uuid::Uuid;

use crate::{
    globals::{
        actions::create_one_block,
        helpers::get_language_profile,
        states::{ProtectedBlock, States},
    },
    key_mappings::{key_contexts::SIDEBAR, mappings::CreateOneBlock},
    libs::{
        tree::{Tree, TreeState, drag::DraggedBlocks, tree},
        tree_view_sidebar::TreeViewSidebar,
    },
    widgets::{
        blocks_tree::build_blocks_tree,
        sidebar::tree::{create_root_tree_list_item, create_tree_list_item},
    },
};

#[derive(Debug)]
pub struct OpenNoteSidebar {
    focus_handle: FocusHandle,
    is_toggled: bool,
    tree_state: Entity<TreeState>,

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
            let Some(uuid) = this.tree_state.read(cx).selected_block else {
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

        // Read TreeState values before the closure to avoid re-entrant read panic
        let dragged_target_block = self.tree_state.read(cx).dragged_target_block;
        let selected_block = self.tree_state.read(cx).selected_block;
        let selected_blocks = self.tree_state.read(cx).selected_blocks.clone();

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

            let is_dragged_over = dragged_target_block == Some(uuid);

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

            let is_selected = selected_block == Some(uuid);
            let is_multi_selected = selected_blocks.contains(&uuid);

            let current_selections = if let Some(dragged) = selected_block {
                vec![dragged]
            } else {
                selected_blocks.iter().copied().collect()
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

impl Focusable for OpenNoteSidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OpenNoteSidebar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Return an empty div to toggle it off,
        // because .is_visible() is just invisible, therefore
        // it won't really disappear the sidebar, therefore,
        // the editor can't take up the rest of the space when sidebar is gone.
        if !self.is_toggled {
            return div();
        }

        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
        let states: &States = cx.global();
        let entity_id = cx.entity_id();

        log::debug!("Refreshing sidebar...");
        log::debug!(
            "Single selected block: {:?}",
            self.tree_state.read(cx).selected_block
        );
        log::debug!(
            "Multi selected blocks: {:?}",
            self.tree_state.read(cx).selected_blocks
        );
        log::debug!("Got {} blocks", states.blocks.read().unwrap().len());

        div()
            .key_context(SIDEBAR)
            .track_focus(&self.focus_handle(cx))
            .h_full() // We need h_full to display the sidebar in full height, but not necessarily size_full
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
                if let Some(block) = this.tree_state.read(cx).selected_block {
                    parent_block_id = Some(block)
                }

                log::debug!("About to create a block under: {:?}", parent_block_id);
                create_one_block(cx, parent_block_id);
                cx.notify();
            }))
    }
}
