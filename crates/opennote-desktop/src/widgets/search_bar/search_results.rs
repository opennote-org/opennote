use std::vec;

use gpui::{ParentElement, SharedString, Styled, WeakEntity};
use gpui_component::{
    IndexPath,
    list::{ListDelegate, ListItem},
    text::Text,
    v_flex,
};

use opennote_core_logics::helpers::run_async_code;
use opennote_data::search::{SearchScope, models::RawSearchResult};
use opennote_embedder::vectorization::send_vectorization;
use opennote_models::{
    block::Block,
    configurations::search::SupportedSearchMethod,
    payload::{Payload, create_query},
};

use crate::{
    globals::{
        actions::route_helpers::{self},
        bootstrap::GlobalApplicationBootStrap,
        states::{ServerStates, States},
    },
    widgets::{pane::helpers::open_block, search_bar::bar::SearchBar},
};

fn create_search_queries(
    query: &str,
    search_method: SupportedSearchMethod,
    bootstrap: &GlobalApplicationBootStrap,
) -> (Option<String>, Option<Vec<f32>>) {
    let query_str = query.to_string();
    let mut query_vector = Vec::new();

    if search_method == SupportedSearchMethod::Semantic {
        let Some(embedders) = &bootstrap.0.embedders else {
            return (None, None);
        };
        let payload = create_query(query);
        let payloads =
            run_async_code(async { send_vectorization(vec![payload], embedders).await.unwrap() });
        query_vector = payloads[0].vector.clone();
    }

    (Some(query_str), Some(query_vector))
}

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
    pub results: Vec<(Block, Payload, RawSearchResult)>,

    /// Raw results from querying the search endpoint
    pub raw_results: Vec<RawSearchResult>,

    pub servers_to_retrieve: Vec<(SharedString, ServerStates)>,

    /// Indicate which query is current active as the user types
    pub active_query_id: usize,

    pub search_bar: WeakEntity<SearchBar>,

    ///
    pub selected_index: Option<IndexPath>,
}

impl SearchResultsList {
    pub fn new(search_bar: WeakEntity<SearchBar>) -> Self {
        Self {
            results: Vec::new(),
            raw_results: Vec::new(),
            servers_to_retrieve: Vec::new(),
            selected_index: None,
            search_bar,
            active_query_id: 0,
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
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<gpui_component::list::ListState<Self>>,
    ) -> Option<Self::Item> {
        self.results
            .get(ix.row)
            .map(|(block, payload, _raw_search_result)| {
                let texts = SharedString::from(payload.texts.clone());
                let search_bar = self.search_bar.clone();

                let content = v_flex().child(Text::String(texts.clone()));

                let block_id = block.id;

                ListItem::new(ix)
                    .selected(Some(ix) == self.selected_index)
                    .h_64()
                    .child(content)
                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                        open_block(cx, block_id, Some(texts.clone()));
                        let _ = search_bar.update(cx, |this, cx| {
                            this.is_toggled = false;
                            cx.notify();
                        });
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
        // Create a query id for the observer to validate
        // if this is the current query.
        self.active_query_id += 1;
        let query_id = self.active_query_id;

        // Cleanup before searching
        self.results.clear();
        self.raw_results.clear();
        self.servers_to_retrieve.clear();

        // Adopt the search method accordingly
        // Retrieve the search method from global state
        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let configurations = bootstrap.get_configurations();

        let states: &States = cx.global();
        let Some(active_pane) = states.get_active_pane(cx) else {
            return gpui::Task::ready(());
        };

        let selected_block_id = active_pane
            .read_with(cx, |this, _cx| this.selected_block_id)
            .unwrap();

        // Determine the blocks to search for
        // When the search scope is userspace, we search across all servers
        // When the search scope is document or collection, we search across the belonging server
        let (servers, block_ids) = match states.get_search_scope() {
            SearchScope::Document => match selected_block_id {
                Some(result) => {
                    let block_ids = vec![result];
                    (states.get_servers_by_block_ids(&block_ids), block_ids)
                }
                None => return gpui::Task::ready(()),
            },
            SearchScope::Collection => {
                // Get the selected block id
                let block_id = match selected_block_id {
                    Some(result) => result,
                    None => return gpui::Task::ready(()),
                };

                // find all blocks that have selected block as their parents
                let block_ids = states.find_block_children_ids(block_id);

                (states.get_servers_by_block_ids(&block_ids), block_ids)
            }
            SearchScope::Userspace => (
                states
                    .get_servers()
                    .iter()
                    .map(|item| (item.0.to_owned(), item.1.to_owned()))
                    .collect(),
                states.get_all_blocks_ids(),
            ),
        };

        // Send the search operation to the background from here
        // Store the raw results to a struct field
        // Retrieve the raw results when raw results are updated
        // Retreive the blocks, then save to the self.results

        let databases = bootstrap.0.databases.clone();
        let search_method = configurations.user.search.default_search_method;
        let top_n = configurations.user.search.top_n;

        let (query_str, query_vector) = create_search_queries(query, search_method, bootstrap);
        if query_str.is_none() && query_vector.is_none() {
            self.results = Vec::new();
            return gpui::Task::ready(());
        }

        for (name, server) in servers.clone().into_iter() {
            let block_ids = block_ids.clone();
            let query_str = query_str.clone();
            let query_vector = query_vector.clone();
            let databases = databases.clone();
            cx.spawn(async move |this, cx| {
                let raw_results = match route_helpers::route_search_blocks(
                    &name,
                    &server,
                    &databases,
                    search_method,
                    block_ids,
                    query_str,
                    query_vector,
                    top_n,
                )
                .await
                {
                    Ok(results) => results,
                    Err(error) => panic!("{}", error),
                };

                let _ = this.update(cx, |this, cx| {
                    // Cancel all old queries as the user types
                    if query_id == this.delegate().active_query_id {
                        this.delegate_mut().raw_results.extend(raw_results);
                        cx.notify();
                    }
                });
            })
            .detach();
        }

        self.servers_to_retrieve = servers;

        gpui::Task::ready(())
    }
}
