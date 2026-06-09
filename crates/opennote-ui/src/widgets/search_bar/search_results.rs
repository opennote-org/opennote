use std::vec;

use gpui::{ParentElement, Styled};
use gpui_component::{
    IndexPath, h_flex,
    label::Label,
    list::{ListDelegate, ListItem},
};

use opennote_core_logics::search::{search_by_keyword, search_by_semantics};
use opennote_embedder::vectorization::send_vectorization;
use opennote_models::{
    block::Block,
    configurations::search::SupportedSearchMethod,
    payload::{Payload, create_query},
};

use crate::{
    globals::{bootstrap::GlobalApplicationBootStrap, states::States},
    widgets::pane::helpers::open_block,
};

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
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> gpui::Task<()> {
        // Adopt the search method accordingly
        // Retrieve the search method from global state
        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let configurations = bootstrap.get_configurations();

        let states: &States = cx.global();
        let Some(active_pane) = states.active_pane.clone() else {
            return gpui::Task::ready(());
        };

        let Some(block_id) = active_pane
            .read_with(cx, |this, _cx| this.selected_block_id)
            .unwrap()
        else {
            return gpui::Task::ready(());
        };

        let databases = &bootstrap.0.databases;
        let raw_results = match configurations.user.search.default_search_method {
            SupportedSearchMethod::Keyword => tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    search_by_keyword(
                        databases,
                        [block_id].to_vec(),
                        query,
                        configurations.user.search.top_n,
                    )
                    .await
                    .unwrap()
                })
            }),
            SupportedSearchMethod::Semantic => tokio::task::block_in_place(|| {
                let Some(embedders) = &bootstrap.0.embedders else {
                    return Vec::new();
                };

                tokio::runtime::Handle::current().block_on(async {
                    let payload = create_query(query);

                    let payloads = send_vectorization(vec![payload], embedders).await.unwrap();

                    search_by_semantics(
                        databases,
                        [block_id].to_vec(),
                        &payloads[0].vector,
                        configurations.user.search.top_n,
                    )
                    .await
                    .unwrap()
                })
            }),
        };

        // TODO: convert raw results to blocks and payloads

        // Filter items based on query
        self.filtered_results = std::mem::take(&mut self.results);

        gpui::Task::ready(())
    }
}
