use std::sync::{Arc, RwLock};

use gpui::{
    AppContext, BorrowAppContext, Context, Entity, EntityId, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render, Styled, Subscription,
    WeakEntity, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    Side,
    button::Button,
    h_flex,
    label::Label,
    list::ListItem,
    menu::ContextMenuExt,
    tree::{Tree, TreeState, tree},
};
use uuid::Uuid;

use crate::{
    globals::{
        actions::{create_one_block, delete_n_blocks},
        helpers::get_language_profile,
        states::{ProtectedBlock, States},
    },
    key_mappings::{
        key_contexts::SIDEBAR,
        mappings::{CreateOneBlock, DeleteBlocks},
    },
    libs::tree_view_sidebar::TreeViewSidebar,
    widgets::blocks_tree::build_blocks_tree,
};

pub struct OpenNoteSidebar {
    focus_handle: FocusHandle,
    is_toggled: bool,
    tree_state: Entity<TreeState>,
    selected_block: Option<Uuid>,
    selected_blocks: Vec<Uuid>,

    _subscriptions: Vec<Subscription>,
}

impl OpenNoteSidebar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut _subscriptions = Vec::new();

        let tree_state = cx.new(|cx| TreeState::new(cx));

        // Watch for changes in States, such as the blocks list
        _subscriptions.push(cx.observe_global::<States>(|_this, cx| {
            cx.notify();
        }));

        _subscriptions.push(cx.observe_self(|_this, cx| {
            cx.notify();
        }));

        _subscriptions.push(cx.observe(&tree_state, |_this, tree_state, cx| {
            let Some(selected) = tree_state.read(cx).selected_entry() else {
                return;
            };

            let Ok(uuid) = Uuid::parse_str(&selected.item().id) else {
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
            });
        }));

        _subscriptions.push(cx.observe(&tree_state, |_this, tree_state, cx| {
            let Some(selected) = tree_state.read(cx).selected_entry() else {
                return;
            };

            let Ok(uuid) = Uuid::parse_str(&selected.item().id) else {
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
            });
        }));

        Self {
            focus_handle: cx.focus_handle(), // obtain a new focus from the global pool for this view
            is_toggled: true,
            tree_state,
            selected_block: None,
            selected_blocks: Vec::new(),
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

    /// TODO: how to display the highlights for multi-selections?
    fn create_sidebar_items(
        &self,
        cx: &mut Context<Self>,
        blocks: Arc<RwLock<Vec<ProtectedBlock>>>,
    ) -> Tree {
        let tree_items = build_blocks_tree(blocks);

        self.tree_state.update(cx, |this, cx| {
            this.set_items(tree_items, cx);
        });

        // We need this to update the sidebar's internal state
        let sidebar = cx.entity();

        tree(
            &self.tree_state,
            move |index, entry, _selected, _window, cx| {
                let label = entry.item().label.clone();
                let id = entry.item().id.clone();
                let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
                let sidebar_entity_delete_blocks = sidebar.clone();
                let sidebar_entity_on_mouse_down = sidebar.clone();

                ListItem::new(index)
                    .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
                    .child(
                        h_flex()
                            .gap_2()
                            .child(label)
                            .on_action(move |_action: &DeleteBlocks, _window, cx| {
                                sidebar_entity_delete_blocks.update(cx, |this, cx| {
                                    let mut to_delete = Vec::new();
                                    let is_multi_selected = this.selected_blocks.is_empty();

                                    if is_multi_selected {
                                        if let Some(block) = this.selected_block.take() {
                                            to_delete.push(block);
                                        }
                                    }

                                    if !is_multi_selected {
                                        to_delete.extend(this.selected_blocks.to_owned());
                                        this.selected_blocks.clear();
                                    }

                                    delete_n_blocks(cx, to_delete);
                                });
                            })
                            .on_mouse_down(gpui::MouseButton::Left, move |event, _window, cx| {
                                sidebar_entity_on_mouse_down.update(cx, |this, _cx| {
                                    let id = Uuid::parse_str(&id).unwrap();
                                    match event.button {
                                        MouseButton::Left => this.selected_block = Some(id),
                                        MouseButton::Right => {
                                            // Multi-selection only happens when the platform key is pressed
                                            this.selected_blocks.push(id);
                                        }
                                        _ => {}
                                    }
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
                // .on_click(|click, window, app| if click.is_right_click() {})
            },
        )
    }

    fn create_new_block_button() -> Button {
        Button::new("workspace_sidebar_create_new_block_button")
            .label("+")
            .on_click(move |click, _window, app_cx| {
                if !click.is_right_click() {
                    // Default to create a root block
                    create_one_block(app_cx, None);
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
        let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();
        let states: &States = cx.global();

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
                            .child(Self::create_new_block_button()),
                    ),
            )
            .on_action(cx.listener(|_this, _action: &CreateOneBlock, _window, cx| {
                create_one_block(cx);
                cx.notify();
            }))
    }
}
