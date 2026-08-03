pub mod helpers;
pub mod tab;

use std::collections::HashMap;

use gpui::{
    Action, Context, Div, Entity, FocusHandle, Focusable, Render, SharedString, Subscription,
    Window, div, prelude::*, px,
};
use gpui_component::{
    Sizable,
    description_list::{DescriptionItem, DescriptionList},
    v_flex,
};
use uuid::Uuid;

use crate::globals::{helpers::get_language_profile, states::States};
use crate::key_mappings::{
    helpers::get_keystrokes_as_shared_string,
    mappings::{CreateOneBlock, OpenNewWindow, ToggleCommandBar, ToggleSearchBar},
};
use crate::libs::tabs::tab_bar::TabBar;
use crate::widgets::{
    editor::Editor,
    pane::tab::{TabState, create_tab_bar_for_blocks},
    sidebar::{OpenNoteSidebar, OpenNoteSidebarEvent},
};

/// A container for 0 to many items that are open in the workspace.
/// Treats all items uniformly via the [`ItemHandle`] trait, whether it's an editor, search results multibuffer, terminal or something else,
/// responsible for managing item tabs, focus and zoom states and drag and drop features.
/// Can be split, see `PaneGroup` for more details.
pub struct Pane {
    pub id: Uuid,

    pub selected_block_id: Option<Uuid>,
    pub opened_block_ids: Vec<Uuid>,
    pub opened_block_states: HashMap<Uuid, TabState>,
    /// The string that will highlighted in the editor
    pub search_string: Option<SharedString>,

    focus_handle: FocusHandle,
    pub(crate) editor: Entity<Editor>,

    _subscriptions: Vec<Subscription>,
}

impl Pane {
    pub fn new(
        cx: &mut Context<Self>,
        window: &mut gpui::Window,
        sidebar: Entity<OpenNoteSidebar>,
    ) -> Self {
        let mut _subscriptions = Vec::new();

        _subscriptions.push(cx.subscribe(&sidebar, move |this, _entity, event, cx| {
            if !this.has_opened_blocks() {
                return;
            };

            match event {
                OpenNoteSidebarEvent::BlocksDeleted(block_ids) => {
                    for id in block_ids {
                        if this.opened_block_ids.contains(id) {
                            this.close_tab(id, cx);
                        }
                    }
                }
            }
        }));

        let pane_ref = cx.weak_entity();

        Self {
            id: Uuid::new_v4(),
            focus_handle: cx.focus_handle(),
            selected_block_id: None,
            search_string: None,
            editor: cx.new(|cx| Editor::new(cx, window, pane_ref)),
            opened_block_ids: Vec::new(),
            opened_block_states: HashMap::new(),
            _subscriptions,
        }
    }

    pub(crate) fn close_tab(&mut self, block_id: &Uuid, cx: &mut Context<Self>) {
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
            self.opened_block_states.remove(block_id);

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
            self.opened_block_states.clear();
            self.selected_block_id = None;

            cx.notify();
        }

        // no tab closing for 0 tabs
    }

    pub fn set_search_string(&mut self, string: SharedString) {
        self.search_string = Some(string)
    }

    /// `self.search_string` will be emptied, once called
    pub fn pop_search_string(&mut self) -> Option<SharedString> {
        self.search_string.take()
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
        self.opened_block_states.insert(
            block_id,
            TabState {
                ..Default::default()
            },
        );
        self.selected_block_id = Some(block_id);
        cx.notify();
    }

    pub fn has_opened_blocks(&self) -> bool {
        !self.opened_block_ids.is_empty()
    }

    /// Switch to the next tab (wrapping around).
    pub fn activate_next_tab(&mut self, cx: &mut Context<Self>) {
        let current_index = match self.acquire_block_index() {
            Some(value) => value,
            None => return,
        };

        let next_index = if current_index + 1 < self.opened_block_ids.len() {
            current_index + 1
        } else {
            0
        };

        self.selected_block_id = Some(self.opened_block_ids[next_index]);
        cx.notify();
    }

    /// Switch to the previous tab (wrapping around).
    pub fn activate_previous_tab(&mut self, cx: &mut Context<Self>) {
        let current_index = match self.acquire_block_index() {
            Some(value) => value,
            None => return,
        };

        let prev_index = if current_index > 0 {
            current_index - 1
        } else {
            self.opened_block_ids.len().saturating_sub(1)
        };

        self.selected_block_id = Some(self.opened_block_ids[prev_index]);
        cx.notify();
    }

    fn acquire_block_index(&mut self) -> Option<usize> {
        let Some(selected_block_id) = self.selected_block_id else {
            return None;
        };
        let current_index = self
            .opened_block_ids
            .iter()
            .position(|id| *id == selected_block_id);
        let Some(current_index) = current_index else {
            return None;
        };
        Some(current_index)
    }

    fn create_commmand_board(cx: &mut Context<'_, Pane>) -> Div {
        let language_profile = get_language_profile(cx).unwrap();

        v_flex().size_full().child(
            div().w_48().my_auto().mx_auto().child(
                DescriptionList::new()
                    .columns(1)
                    .bordered(false)
                    .large()
                    .children([
                        DescriptionItem::new(language_profile["search"].to_string()).value(
                            get_keystrokes_as_shared_string(cx, ToggleSearchBar.boxed_clone())
                                .unwrap_or("".into()),
                        ),
                        DescriptionItem::new(language_profile["commands"].to_string()).value(
                            get_keystrokes_as_shared_string(cx, ToggleCommandBar.boxed_clone())
                                .unwrap_or("".into()),
                        ),
                        DescriptionItem::new(language_profile["new_note"].to_string()).value(
                            get_keystrokes_as_shared_string(cx, CreateOneBlock.boxed_clone())
                                .unwrap_or("".into()),
                        ),
                        DescriptionItem::new(language_profile["new_window"].to_string()).value(
                            get_keystrokes_as_shared_string(cx, OpenNewWindow.boxed_clone())
                                .unwrap_or("".into()),
                        ),
                    ]),
            ),
        )
    }

    fn update_editor_with_selected_block(&mut self, cx: &mut Context<'_, Pane>) {
        if let Some(selected_block_id) = self.selected_block_id {
            let states: &States = cx.global();

            let block = states.get_block(&selected_block_id);

            if let Some(block) = block {
                let block = block.to_owned();
                let search_string = self.pop_search_string();

                self.editor.update(cx, |this, cx| {
                    // The backend is always the source of truth.
                    // We fetch the block from the backend with the current uuid.
                    this.register_block(cx, block);
                    this.register_highlighted_text(search_string);
                    cx.notify();
                });
            }
        }
    }
}

impl Focusable for Pane {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Pane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // We need flex_1 to let the editor to take up the whole space after sidebar disappeared.
        // The min_w is to prevent Velotype from shifting its document content to the right.
        let base_div = div().flex_1().min_w(px(0.0)).flex_col();

        // Display search bar, command bar, new doc
        // and their keyboard shortcuts
        if self.opened_block_ids.is_empty() {
            return Self::create_commmand_board(cx);
        }

        let pane_reference = cx.weak_entity();
        let pane_id = self.id;

        let tabs: TabBar = create_tab_bar_for_blocks(
            cx,
            pane_reference,
            pane_id,
            &self.opened_block_ids,
            self.selected_block_id,
            &self.opened_block_states,
        );

        // Open editor only when there is an active block
        if self.selected_block_id.is_none() {
            return base_div.child(tabs);
        };

        self.update_editor_with_selected_block(cx);

        base_div
            .h_full()
            .child(tabs)
            .child(div().h_full().child(self.editor.clone()))
    }
}
