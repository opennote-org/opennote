pub mod helpers;
pub mod tab;

mod editor;
mod observations;
mod subscriptions;

use gpui_kit::component::{
    ActiveTheme, Sizable,
    description_list::{DescriptionItem, DescriptionList},
    v_flex,
};
use gpui_kit::{
    Action, Context, Div, Entity, EventEmitter, FocusHandle, Focusable, Render, SharedString,
    Subscription, Window, div, prelude::*, px,
};
use uuid::Uuid;

use opennote_velotype::editor::Editor;

use crate::{
    globals::{helpers::get_language_profile, tasks::tracker::TaskTracker},
    key_mappings::{
        helpers::get_keystrokes_as_shared_string,
        mappings::{CreateOneBlock, OpenNewWindow, ToggleCommandBar, ToggleSearchBar},
    },
    libs::tabs::tab_bar::TabBar,
    widgets::{
        pane::{
            observations::{observe_chunk_block, observe_theme_change},
            tab::{TabStates, create_tab_bar_for_blocks},
        },
        sidebar::{OpenNoteSidebar, OpenNoteSidebarEvent},
    },
};

pub struct Pane {
    pub id: Uuid,

    /// The block that is registered as a selected block.
    /// Operations will most likely be performed on this one.
    pub selected_block_id: Option<Uuid>,

    pub opened_block_ids: Vec<Uuid>,
    pub opened_tab_states: TabStates,
    /// The string that will highlighted in the editor
    pub search_string: Option<SharedString>,

    focus_handle: FocusHandle,

    /// The active editor.
    pub(crate) editor: Option<Entity<opennote_velotype::editor::Editor>>,

    _subscriptions: Vec<Subscription>,
}

impl Pane {
    pub fn new(
        cx: &mut Context<Self>,
        window: &mut gpui_kit::Window,
        sidebar: Entity<OpenNoteSidebar>,
    ) -> Self {
        let mut _subscriptions = Vec::new();

        _subscriptions.push(cx.subscribe_in(
            &sidebar,
            window,
            move |this, _entity, event, window, cx| {
                if this.get_active_editor().is_none() {
                    return;
                }

                match event {
                    OpenNoteSidebarEvent::BlocksDeleted(block_ids) => {
                        for id in block_ids {
                            if this.opened_block_ids.contains(id) {
                                this.close_tab(id, cx, window);
                            }
                        }
                    }
                }
            },
        ));

        // Get updates from the normal task scheduler
        _subscriptions.push(cx.observe_global_in::<TaskTracker>(window, observe_chunk_block));

        _subscriptions.push(cx.observe_window_appearance(window, observe_theme_change));

        Self {
            id: Uuid::new_v4(),
            focus_handle: cx.focus_handle(),
            selected_block_id: None,
            search_string: None,
            editor: None,
            opened_block_ids: Vec::new(),
            opened_tab_states: TabStates::new(),
            _subscriptions,
        }
    }

    pub(crate) fn close_tab(
        &mut self,
        block_id: &Uuid,
        cx: &mut Context<Self>,
        window: &mut gpui_kit::Window,
    ) {
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
            self.opened_tab_states.remove_tab_state(block_id, window);

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
                    self.open_or_activate_tab(*block_to_be_selected, cx, window);
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
            self.opened_tab_states.remove_all_tab_state(window);
            self.selected_block_id = None;
            self.editor = None;

            // Request releasing focus from Pane
            cx.emit(PaneEvent::ReleaseFocus);

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

    /// Open a new tab in this Pane.
    /// Activate an existing one if that tab exists already.
    pub fn open_or_activate_tab(
        &mut self,
        block_id: Uuid,
        cx: &mut Context<Self>,
        window: &mut gpui_kit::Window,
    ) {
        // Create a new editor if no editor is openning,
        // otherwise, swap in the preserved editor.
        let editor_to_open = match self.opened_tab_states.get_tab_state_editor(&block_id) {
            Some(result) => result,
            None => {
                self.opened_block_ids.push(block_id);
                self.opened_tab_states
                    .create_tab_state(&block_id, cx, window);
                self.opened_tab_states
                    .get_tab_state_editor(&block_id)
                    .unwrap()
            }
        };

        // Move the focus to the editor.
        editor_to_open.update(cx, |this, cx| {
            this.request_focus(cx);
        });

        self.editor = Some(editor_to_open);
        self.selected_block_id = Some(block_id);

        // Request focus from Pane
        cx.emit(PaneEvent::RequestFocus);

        cx.notify();
    }

    pub fn get_active_editor(&self) -> Option<Entity<Editor>> {
        self.editor.clone()
    }

    /// Switch to the next tab (wrapping around).
    pub fn activate_next_tab(&mut self, cx: &mut Context<Self>, window: &mut gpui_kit::Window) {
        let current_index = match self.acquire_block_index() {
            Some(value) => value,
            None => return,
        };

        let next_index = if current_index + 1 < self.opened_block_ids.len() {
            current_index + 1
        } else {
            0
        };

        let block_id = self.opened_block_ids[next_index];

        self.selected_block_id = Some(block_id);
        self.open_or_activate_tab(block_id, cx, window);
    }

    /// Switch to the previous tab (wrapping around).
    pub fn activate_previous_tab(&mut self, cx: &mut Context<Self>, window: &mut gpui_kit::Window) {
        let current_index = match self.acquire_block_index() {
            Some(value) => value,
            None => return,
        };

        let prev_index = if current_index > 0 {
            current_index - 1
        } else {
            self.opened_block_ids.len().saturating_sub(1)
        };

        let block_id = self.opened_block_ids[prev_index];

        self.selected_block_id = Some(block_id);
        self.open_or_activate_tab(block_id, cx, window);
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

        v_flex()
            .relative()
            .size_full()
            .child(
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
            .child(
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .text_xs()
                    .text_color(cx.theme().description_list_label_foreground)
                    .child(concat!("v", env!("CARGO_PKG_VERSION"))),
            )
    }

    fn apply_highlighted_text(
        &mut self,
        cx: &mut Context<Self>,
        highlighted_text: Option<SharedString>,
    ) {
        if let Some(editor) = &self.editor {
            editor.update(cx, |this, cx| {
                if let Some(highlighted_text) = highlighted_text {
                    this.highlight_search_result(cx, highlighted_text.into());
                }
            });
        }
    }
}

impl Focusable for Pane {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[derive(Debug)]
pub enum PaneEvent {
    // This happens when the focus needs to drop from this component
    ReleaseFocus,
    RequestFocus,
}

impl EventEmitter<PaneEvent> for Pane {}

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
            &self.opened_tab_states,
        );

        let highlighted_text = self.pop_search_string();
        self.apply_highlighted_text(cx, highlighted_text);

        base_div
            .h_full()
            .child(tabs)
            .child(div().h_full().child(self.render_editor(cx)))
    }
}
