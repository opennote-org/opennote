use std::{
    collections::{HashMap, HashSet},
    vec,
};

use gpui::{Context, Entity, Subscription};
use gpui_component::list::ListState;
use uuid::Uuid;

use opennote_data::search::models::RawSearchResult;
use opennote_models::{payload::Payload, query::BlockQuery};

use crate::{
    globals::{actions::route_helpers::route_read_blocks, bootstrap::GlobalApplicationBootStrap},
    widgets::search_bar::{bar::SearchBar, search_results::SearchResultsList},
};

pub fn observe_search_result_list(
    cx: &mut Context<'_, SearchBar>,
    search_results_list: &Entity<ListState<SearchResultsList>>,
) -> Subscription {
    cx.observe(
        search_results_list,
        move |_this, search_results_list, cx| {
            // Early return if there are no raw search results
            if search_results_list
                .read(cx)
                .delegate()
                .raw_results
                .is_empty()
            {
                return;
            }

            let (raw_results, servers) = search_results_list.update(cx, |this, _cx| {
                let raw_results = std::mem::take(&mut this.delegate_mut().raw_results);
                let servers = std::mem::take(&mut this.delegate_mut().servers_to_retrieve);

                (raw_results, servers)
            });

            let bootstrap: &GlobalApplicationBootStrap = cx.global();
            let databases = bootstrap.0.databases.clone();

            let mut results: HashMap<Uuid, Vec<RawSearchResult>> = HashMap::new();

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

            for (name, server) in servers {
                let databases = databases.clone();
                let block_ids = block_ids.clone();
                let mut results = results.clone();
                cx.spawn(async move |this, cx| {
                    let name = name;
                    let server = server;
                    let filter = BlockQuery::ByIds(block_ids.iter().cloned().collect());
                    let blocks = route_read_blocks(&name, &server, &databases, &filter, false)
                        .await
                        .unwrap();

                    for block in blocks {
                        let mut block = block;
                        let payloads = std::mem::take(&mut block.payloads);
                        let mut payloads: HashMap<Uuid, Payload> =
                            payloads.into_iter().map(|item| (item.id, item)).collect();

                        if let Some(need_payload_ids) = results.remove(&block.id) {
                            for need_payload_id in need_payload_ids {
                                let _ = this.update(cx, |this, cx| {
                                    this.search_results_list.update(cx, |this, cx| {
                                        this.delegate_mut().results.push((
                                            block.clone(),
                                            payloads.remove(&need_payload_id.payload_id).unwrap(),
                                            need_payload_id,
                                        ));

                                        this.delegate_mut()
                                            .results
                                            .sort_by(|a, b| b.2.score.total_cmp(&a.2.score));

                                        cx.notify();
                                    });
                                });
                            }
                        }
                    }
                })
                .detach();
            }
        },
    )
}
