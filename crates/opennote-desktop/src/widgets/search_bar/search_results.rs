use std::{
    collections::{HashMap, HashSet},
    vec,
};

use gpui::{ParentElement, SharedString, Styled, WeakEntity};
use gpui_component::{
    IndexPath,
    list::{ListDelegate, ListItem},
    text::Text,
    v_flex,
};

use opennote_data::{
    database::enums::BlockQuery,
    search::{SearchScope, models::RawSearchResult},
};
use opennote_embedder::vectorization::send_vectorization;
use opennote_models::{
    block::Block,
    configurations::search::SupportedSearchMethod,
    payload::{Payload, create_query},
};
use uuid::Uuid;

use crate::{
    globals::{
        actions::route_helpers::{self, route_read_blocks},
        bootstrap::GlobalApplicationBootStrap,
        helpers::run_async_code,
        states::States,
    },
    widgets::{pane::helpers::open_block, search_bar::bar::SearchBar},
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
    pub results: Vec<(Block, Payload, RawSearchResult)>,

    pub search_bar: WeakEntity<SearchBar>,

    ///
    pub selected_index: Option<IndexPath>,
}

impl SearchResultsList {
    pub fn new(search_bar: WeakEntity<SearchBar>) -> Self {
        Self {
            results: Vec::new(),
            selected_index: None,
            search_bar,
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
        log::debug!(
            "Search results ranking: {:?}",
            &self
                .results
                .iter()
                .map(|item| item.2.score)
                .collect::<Vec<f32>>()
        );

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
        // Adopt the search method accordingly
        // Retrieve the search method from global state
        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let configurations = bootstrap.get_configurations();

        let states: &States = cx.global();
        let Some(active_pane) = states.active_pane.clone() else {
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

        let databases = &bootstrap.0.databases;
        let search_method = configurations.user.search.default_search_method;
        let top_n = configurations.user.search.top_n;

        let raw_results = run_async_code(async {
            let query_str = Some(query.to_string());
            let mut query_vector = None;

            if search_method == SupportedSearchMethod::Semantic {
                let Some(embedders) = &bootstrap.0.embedders else {
                    return Vec::new();
                };
                let payload = create_query(query);
                let payloads = send_vectorization(vec![payload], embedders).await.unwrap();
                query_vector = Some(payloads[0].vector.clone());
            }

            let mut results = Vec::new();
            let mut handles = Vec::new();
            for (name, server) in servers.iter() {
                handles.push(route_helpers::route_search_blocks(
                    &name,
                    &server,
                    &databases,
                    search_method,
                    block_ids.clone(),
                    query_str.clone(),
                    query_vector.clone(),
                    top_n,
                ));
            }
            let gathered = futures::future::join_all(handles).await;
            for result in gathered {
                results.extend(result.unwrap());
            }

            results
        });

        // TODO: convert raw results to blocks and payloads
        let mut results: HashMap<Uuid, Vec<RawSearchResult>> = HashMap::new();
        let blocks = run_async_code(async {
            // Get block ids for retrieving them
            let mut block_ids = HashSet::new();

            // Also save them to a hash map for pairing
            for raw_result in raw_results {
                let block_id = raw_result.block_id;

                block_ids.insert(block_id);

                // Insert payload id when block is there
                if let Some(payloads) = results.get_mut(&block_id) {
                    payloads.push(raw_result);
                    continue;
                }

                // Insert payload id and block id when block is not there
                if results.get(&block_id).is_none() {
                    results.insert(block_id, vec![raw_result]);
                }
            }

            let mut block_handles = Vec::new();
            for (name, server) in &servers {
                block_handles.push(async {
                    let filter = BlockQuery::ByIds(block_ids.iter().cloned().collect());
                    route_read_blocks(name, server, databases, &filter).await
                });
            }
            let blocks_results = futures::future::join_all(block_handles).await;
            let mut blocks = Vec::new();
            for res in blocks_results {
                blocks.extend(res.unwrap());
            }

            blocks
        });

        // Pair payloads with their blocks
        self.results.clear();
        for block in blocks {
            let mut block = block;
            let payloads = std::mem::take(&mut block.payloads);
            let mut payloads: HashMap<Uuid, Payload> =
                payloads.into_iter().map(|item| (item.id, item)).collect();

            if let Some(need_payload_ids) = results.remove(&block.id) {
                for need_payload_id in need_payload_ids {
                    self.results.push((
                        block.clone(),
                        payloads.remove(&need_payload_id.payload_id).unwrap(),
                        need_payload_id,
                    ));
                }
            }
        }

        self.results.sort_by(|a, b| b.2.score.total_cmp(&a.2.score));

        gpui::Task::ready(())
    }
}
