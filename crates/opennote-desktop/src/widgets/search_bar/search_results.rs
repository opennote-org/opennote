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

use opennote_core_logics::{
    block::read_blocks,
    search::{search_by_keyword, search_by_semantics},
};
use opennote_data::{database::enums::BlockQuery, search::SearchScope};
use opennote_embedder::vectorization::send_vectorization;
use opennote_models::{
    block::Block,
    configurations::search::SupportedSearchMethod,
    payload::{Payload, create_query},
};
use uuid::Uuid;

use crate::{
    globals::{bootstrap::GlobalApplicationBootStrap, helpers::run_async_code, states::States},
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
    pub results: Vec<(Block, Payload)>,

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
        self.results.get(ix.row).map(|(block, payload)| {
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
        let block_ids = match states.get_search_scope() {
            SearchScope::Document => match selected_block_id {
                Some(result) => vec![result],
                None => return gpui::Task::ready(()),
            },
            SearchScope::Collection => {
                // Get the selected block id
                let block_id = match selected_block_id {
                    Some(result) => result,
                    None => return gpui::Task::ready(()),
                };

                // find all blocks that have selected block as their parents
                states
                    .blocks
                    .iter()
                    .filter_map(|(_, block)| {
                        if block.parent_id == Some(block_id) {
                            Some(block.id)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            SearchScope::Userspace => states.blocks.keys().map(|item| item.to_owned()).collect(),
        };

        let databases = &bootstrap.0.databases;
        let raw_results = match configurations.user.search.default_search_method {
            SupportedSearchMethod::Keyword => run_async_code(async {
                search_by_keyword(
                    databases,
                    block_ids,
                    query,
                    configurations.user.search.top_n,
                )
                .await
                .unwrap()
            }),
            SupportedSearchMethod::Semantic => run_async_code(async {
                let Some(embedders) = &bootstrap.0.embedders else {
                    return Vec::new();
                };

                let payload = create_query(query);

                let payloads = send_vectorization(vec![payload], embedders).await.unwrap();

                search_by_semantics(
                    databases,
                    block_ids,
                    &payloads[0].vector,
                    configurations.user.search.top_n,
                )
                .await
                .unwrap()
            }),
        };

        // TODO: convert raw results to blocks and payloads
        let mut results: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let blocks = run_async_code(async {
            // Get block ids for retrieving them
            let mut block_ids = HashSet::new();

            // Also save them to a hash map for pairing
            for raw_result in raw_results {
                let block_id = raw_result.block_id;

                block_ids.insert(block_id);

                // Insert payload id when block is there
                if let Some(payloads) = results.get_mut(&block_id) {
                    payloads.push(raw_result.payload_id);
                    continue;
                }

                // Insert payload id and block id when block is not there
                if results.get(&block_id).is_none() {
                    results.insert(block_id, vec![raw_result.payload_id]);
                }
            }

            read_blocks(
                databases,
                &BlockQuery::ByIds(block_ids.into_iter().collect()),
            )
            .await
            .unwrap()
        });

        // Pair payloads with their blocks
        self.results.clear();
        for block in blocks {
            let mut block = block;
            let payloads = std::mem::take(&mut block.payloads);
            let mut payloads: HashMap<Uuid, Payload> =
                payloads.into_iter().map(|item| (item.id, item)).collect();

            if let Some(payload_ids) = results.remove(&block.id) {
                for id in payload_ids {
                    self.results
                        .push((block.clone(), payloads.remove(&id).unwrap()));
                }
            }
        }

        gpui::Task::ready(())
    }
}
