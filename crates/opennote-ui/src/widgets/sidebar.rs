use std::sync::{Arc, RwLock};

use gpui::{
    AppContext, BorrowAppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Subscription, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    Side,
    button::Button,
    h_flex,
    label::Label,
    list::ListItem,
    tree::{Tree, TreeState, tree},
};
use uuid::Uuid;

use crate::{
    globals::{
        actions::create_one_block,
        helpers::get_language_profile,
        states::{ProtectedBlock, States},
    },
    key_mappings::{key_contexts::SIDEBAR, mappings::CreateOneBlock},
    libs::tree_view_sidebar::TreeViewSidebar,
    widgets::blocks_tree::build_blocks_tree,
};

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

    fn create_sidebar_items(
        &self,
        cx: &mut Context<Self>,
        blocks: Arc<RwLock<Vec<ProtectedBlock>>>,
    ) -> Tree {
        let tree_items = build_blocks_tree(blocks);

        self.tree_state.update(cx, |this, cx| {
            this.set_items(tree_items, cx);
        });

        tree(&self.tree_state, |ix, entry, _selected, _window, _cx| {
            let item = entry.item();

            ListItem::new(ix)
                // .selected(selected)
                .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
                .child(h_flex().gap_2().child(item.label.clone()))
        })
    }

    fn create_new_block_button() -> Button {
        Button::new("workspace_sidebar_create_new_block_button")
            .label("+")
            .on_click(move |click, _window, app_cx| {
                if !click.is_right_click() {
                    create_one_block(app_cx);
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
