use gpui::{
    AppContext, BorrowAppContext, Context, ElementId, Entity, ParentElement, Render, SharedString,
    Styled, Subscription, div,
};
use gpui_component::{
    IconName, Selectable, Sizable,
    button::{Button, ButtonRounded, ButtonVariants},
    tab::{Tab, TabBar},
};

use opennote_models::block::Block;
use uuid::Uuid;

use crate::{globals::states::States, widgets::editor::Editor};

pub struct EditorTabs {
    openned_blocks: Vec<Block>,
    editor: Entity<Editor>,

    _subscriptions: Vec<Subscription>,
}

impl EditorTabs {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        let mut _subscriptions = Vec::new();

        // However, we can use observe_global_in to use `window`, if needed
        _subscriptions.push(cx.observe_global::<States>(|this, cx| {
            log::debug!("Editor refreshes because the global state had changed");

            cx.read_global::<States, ()>(|states, _cx| {
                // The active block sticks with the global active block
                if let Some(active_block_id) = &states.active_block_id {
                    // Skip if the block has been openned
                    for openned_block in this.openned_blocks.iter() {
                        if openned_block.id == *active_block_id {
                            return;
                        }
                    }

                    let block = states.get_active_block();
                    if let Some(block) = block {
                        this.openned_blocks.push(block.clone());
                    }
                }
            });

            cx.notify();
        }));

        Self {
            editor: cx.new(|cx| Editor::new(cx, window)),
            openned_blocks: Vec::new(),
            _subscriptions,
        }
    }

    fn close_tab(&mut self, block_id: Uuid, cx: &mut Context<Self>) {
        let states: &States = cx.global();
        let active_block = states.get_active_block();

        // if we have multiple tabs openning
        if self.openned_blocks.len() > 1 {
            // Remove the closed block from the openned blocks,
            // while also retain an index for moving the focus to the prevoius one
            let mut removed_index = 0;
            for (index, block) in self.openned_blocks.iter().enumerate() {
                if block.id == block_id && index != 0 {
                    removed_index = index;
                    break;
                }
            }

            self.openned_blocks.remove(removed_index);

            // Move the focus to the previous tab / block
            if let Some(active_block) = active_block {
                let index_to_focus = removed_index - 1;

                let Some(move_to_block) = self.openned_blocks.get(index_to_focus) else {
                    return;
                };

                // Move the focus only when the active block has been closed
                if active_block.id == block_id {
                    cx.update_global::<States, ()>(|this, _cx| {
                        this.active_block_id = Some(move_to_block.id);
                    });
                }
            }

            cx.notify();

            // Prevent triggering the 1 tab case when
            // the openned tabs become 1 after the tab closing
            return;
        }

        // if we only have 1 tab openning
        if self.openned_blocks.len() == 1 {
            self.openned_blocks.clear();
            cx.update_global::<States, ()>(|this, _cx| {
                this.active_block_id = None;
            });

            cx.notify();
        }

        // no tab closing for 0 tabs
    }
}

impl Render for EditorTabs {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let base_div = div().flex_1().flex_col(); // We need flex_1 to let the editor to take up the whole space after sidebar disappeared

        if self.openned_blocks.is_empty() {
            return base_div.child("No documents yet");
        }

        let tabs = TabBar::new("tabs")
            .children(self.openned_blocks.iter().map(|item| {
                let id = item.id;
                let mut selected = false;

                let states: &States = cx.global();

                if let Some(active_block_id) = states.active_block_id {
                    if active_block_id == id {
                        selected = true;
                    }
                }

                // TODO: can't see the close button icon; click to refocus; 
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
                            })),
                    )
            }))
            .on_click(|index, window, cx| {});

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

        base_div.h_full().child(tabs).child(editor)
    }
}
