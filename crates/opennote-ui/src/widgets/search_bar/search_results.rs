use gpui::{ParentElement, Styled};
use gpui_component::{
    IndexPath, h_flex,
    label::Label,
    list::{ListDelegate, ListItem},
};

use opennote_models::{block::Block, payload::Payload};

use crate::widgets::pane::helpers::open_block;


/// Collect all available gpui actions / key bindings in this app
///
/// TODO:
/// - Store blocks and the search result payload as result
/// - On click a result, open the editor to the payload position of that block
/// - If the editor had opened, switch to that editor instead
///
/// - Provide two searches, semantic and keyword
/// - Search methods' block_ids is determined by the current context
pub struct SearchResultsList {
    /// Searched block and the specific payload contains the result
    pub results: Vec<(Block, Payload)>,

    ///
    pub filtered_results: Vec<(Block, Payload)>,

    ///
    pub selected_index: Option<IndexPath>,
}

impl SearchResultsList {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            filtered_results: Vec::new(),
            selected_index: None,
        }
    }
}

impl ListDelegate for SearchResultsList {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.results.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> Option<Self::Item> {
        self.results.get(ix.row).map(|(block, payload)| {
            let content = h_flex()
                .items_center()
                .justify_between()
                .child(Label::new(block.get_title()))
                .child(payload.texts.clone());

            let block_id = block.id;

            ListItem::new(ix)
                .selected(Some(ix) == self.selected_index)
                .child(content)
                .on_click(cx.listener(move |_this, _, window, cx| {
                    open_block(cx, block_id);
                }))
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> gpui::Task<()> {
        // // Adopt the search method accordingly
        // // Retrieve the search method from global state
        // match  {

        // }

        // // Filter items based on query
        // self.filtered_results = self
        //     .results
        //     .iter()
        //     .filter(|(block, payload)| {
        //         action.name().to_lowercase().contains(&query.to_lowercase())
        //     })
        //     .map(|(action, key_binding)| (action.boxed_clone(), key_binding.to_owned()))
        //     .collect();

        gpui::Task::ready(())
    }
}
