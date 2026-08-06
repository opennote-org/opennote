mod blocks_tree;
mod tab;

pub mod tree;

use std::collections::HashMap;

use anyhow::Result;
use gpui::{
    AppContext, BorrowAppContext, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Render, SharedString, Styled,
    Subscription, Window, div,
};
use gpui_component::{ActiveTheme, Side, button::Button, h_flex, label::Label};
use uuid::Uuid;

use opennote_models::{block::Block, constants::LOCAL_SERVER_NAME};

use crate::{
    globals::{actions::create_one_block, helpers::get_language_profile, states::States},
    key_mappings::key_contexts::SIDEBAR,
    libs::{
        tabs::{drag::DraggedItem, tab_bar::TabBar},
        tree::{Tree, TreeState, tree},
        tree_view_sidebar::{DEFAULT_WIDTH, TreeViewSidebar},
    },
    widgets::{
        pane::helpers::open_block,
        sidebar::{
            blocks_tree::build_blocks_tree,
            tab::create_sidebar_tabbar,
            tree::{create_root_tree_list_item, create_tree_list_item},
        },
    },
};

#[derive(Debug)]
struct BlockState {
    pub has_expanded: bool,
}

#[derive(Debug, Clone)]
pub enum OpenNoteSidebarEvent {
    BlocksDeleted(Vec<Uuid>),
}

#[derive(Debug)]
pub struct OpenNoteSidebar {
    focus_handle: FocusHandle,
    is_toggled: bool,
    tree_states: HashMap<SharedString, Entity<TreeState>>,
    blocks_state: HashMap<Uuid, BlockState>,

    mouse_position: Option<Point<Pixels>>,

    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenNoteSidebarEvent> for OpenNoteSidebar {}

impl OpenNoteSidebar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut _subscriptions = Vec::new();
        let mut tree_states = HashMap::new();

        // Create tree states for each server
        tree_states.insert(
            SharedString::from(LOCAL_SERVER_NAME),
            cx.new(|cx| TreeState::new(cx)),
        );

        let _ = cx.update_global::<States, ()>(|states, cx| {
            for (name, _remote_server_states) in states.get_servers() {
                tree_states.insert(name.clone(), cx.new(|cx| TreeState::new(cx)));
            }
        });

        // Watch for changes in States, such as the blocks list.
        //
        // Please avoid using update_global method as much as possible,
        // otherwise, GPUI will keep refreshing because update_global will trigger the observer.
        _subscriptions.push(cx.observe_global::<States>(|_this, cx| {
            cx.notify();
        }));

        Self {
            focus_handle: cx.focus_handle(), // obtain a new focus from the global pool for this view
            is_toggled: true,
            tree_states,
            blocks_state: HashMap::new(),
            mouse_position: None,
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

    pub fn get_tree_state(&self, server_name: &SharedString) -> Option<Entity<TreeState>> {
        if let Some(tree_state) = self.tree_states.get(server_name) {
            return Some(tree_state.clone());
        }

        None
    }

    pub fn get_tree_focus_handle(
        &self,
        cx: &Context<Self>,
        server_name: &SharedString,
    ) -> Option<FocusHandle> {
        if let Some(tree_state) = self.tree_states.get(server_name) {
            return Some(tree_state.read(cx).focus_handle(cx));
        }

        None
    }

    /// Use .unwrap by default. Make sure the input is a valid uuid string
    fn convert_str_to_uuid(str: &str) -> Result<Uuid> {
        Ok(Uuid::parse_str(str)?)
    }

    fn create_sidebar_items(
        &mut self,
        cx: &mut Context<Self>,
        tree_state: Entity<TreeState>,
        blocks: Vec<Block>,
    ) -> Tree {
        let tree_items = build_blocks_tree(blocks, &mut self.blocks_state);

        tree_state.update(cx, |this, cx| {
            this.set_items(tree_items, cx);
        });

        // Read TreeState values before the closure to avoid re-entrant read panic
        let dragged_target_block = tree_state.read(cx).dragged_target_block;
        let selected_block = tree_state.read(cx).selected_block;
        let selected_blocks = tree_state.read(cx).selected_blocks.clone();

        // We need this to update the sidebar's internal state
        let sidebar = cx.entity();
        let tree_state_clone = tree_state.clone();

        let tree = tree(&tree_state_clone, move |index, entry, _window, cx| {
            let id = entry.item().id.clone(); // This is a stringified uuid of a block
            let label = entry.item().label.clone();
            let language_profile = get_language_profile(cx).unwrap();
            let sidebar = sidebar.clone();
            let tree_state = tree_state.clone();

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
                    tree_state,
                    sidebar,
                    is_dragged_over,
                );
            }

            let is_selected = selected_block == Some(uuid);
            let is_multi_selected = selected_blocks.contains(&uuid);
            let has_children = !entry.item().children.is_empty();

            let current_selections = if let Some(dragged) = selected_block {
                vec![dragged]
            } else {
                selected_blocks.iter().copied().collect()
            };

            let dragged_block = DraggedItem {
                block_id: Some(uuid),
                label: Some(label.clone()),
                selections: current_selections,
                ..Default::default()
            };

            create_tree_list_item(
                index,
                entry,
                label,
                id,
                uuid,
                language_profile,
                sidebar,
                tree_state,
                is_selected,
                is_multi_selected,
                is_dragged_over,
                dragged_block,
                has_children,
            )
        });

        tree
    }

    fn create_new_block_button(entity_id: EntityId) -> Button {
        Button::new("workspace_sidebar_create_new_block_button")
            .label("+")
            .on_click(move |click, window, app_cx| {
                if !click.is_right_click() {
                    // Default to create a root block
                    create_one_block(window, app_cx, None);
                    app_cx.notify(entity_id);
                }
            })
    }

    pub fn handle_block_creation(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        tree_state: Entity<TreeState>,
    ) {
        let mut parent_block_id = None;
        if let Some(block) = tree_state.read(cx).selected_block {
            parent_block_id = Some(block)
        }

        create_one_block(window, cx, parent_block_id);
        cx.notify();
    }

    pub fn handle_block_open(
        &self,
        block_id: Uuid,
        cx: &mut Context<Self>,
        tree_state: Entity<TreeState>,
    ) {
        // Select the block
        tree_state.update(cx, |this, cx| {
            this.selected_blocks.clear();
            this.selected_block = None;

            open_block(cx, block_id, None);
            cx.notify();
        });

        cx.notify();
        return;
    }
}

impl Focusable for OpenNoteSidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OpenNoteSidebar {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Return an empty div to toggle it off,
        // because .is_visible() is just invisible, therefore
        // it won't really disappear the sidebar, therefore,
        // the editor can't take up the rest of the space when sidebar is gone.
        if !self.is_toggled {
            return div();
        }

        let language_profile = get_language_profile(cx).unwrap();
        let entity_id = cx.entity_id();
        let window_id = window.window_handle().window_id();

        let (active_server_name, blocks, remote_server_tab_bar) =
            cx.read_global::<States, (SharedString, Vec<Block>, TabBar)>(|states, _cx| {
                let active_server_name = states.get_active_server_name(window_id);
                let remote_server_tab_bar =
                    create_sidebar_tabbar(active_server_name.clone(), states.get_servers());
                let blocks = states.get_all_blocks_by_server(&active_server_name);

                (active_server_name, blocks, remote_server_tab_bar)
            });

        div()
            .key_context(SIDEBAR)
            .track_focus(&self.focus_handle(cx))
            .w(DEFAULT_WIDTH)
            .border_color(cx.theme().sidebar_border) // Together with border_r_l to create a border line
            .border_r_1()
            .h_full() // We need h_full to display the sidebar in full height, but not necessarily size_full
            .child(remote_server_tab_bar)
            .child(
                TreeViewSidebar::new(Side::Left)
                    .child(
                        self.create_sidebar_items(
                            cx,
                            self.get_tree_state(&SharedString::new(active_server_name))
                                .unwrap(),
                            blocks,
                        ),
                    )
                    .header(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(Label::new(&language_profile["sidebar_title"]).text_xl())
                            .child(Self::create_new_block_button(entity_id)),
                    ),
            )
    }
}
