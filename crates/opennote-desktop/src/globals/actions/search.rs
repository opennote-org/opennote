use std::collections::{HashMap, HashSet};

use anyhow::Result;
use gpui_kit::SharedString;
use uuid::Uuid;

use opennote_data::Databases;
use opennote_models::{
    block::Block,
    configurations::{fields::search::SupportedSearchMethod, system::SystemConfigurations},
    payload::Payload,
    query::BlockQuery,
    search::RawSearchResult,
    traits::summarization::Summarization,
};

use crate::globals::{
    actions::route_helpers::{route_read_blocks, route_search_blocks},
    language::LanguageProfile,
    states::server_registry::ServerStates,
};

pub struct SearchResult {
    pub block: Block,
    pub payload: Payload,
    pub raw: RawSearchResult,
    pub server: SharedString,
    pub breadcrumb: SharedString,
}

impl Summarization for SearchResult {
    fn summarize(&self) -> String {
        format!(
            "Title: {}\nBlock ID: {}\nPayload ID: {}\nServer: {}\nBreadcrumb: {}\nScore: {}\nContent: {}\n\n",
            self.block.get_title(),
            self.block.id,
            self.payload.id,
            self.server,
            self.breadcrumb,
            self.raw.score,
            self.payload.texts,
        )
    }
}

#[derive(Clone, Copy)]
pub enum SearchStatus {
    Idle,
    Searching,
    Empty,
    SemanticUnavailable,
    PartialFailure,
    Matches(usize),
}

impl SearchStatus {
    pub fn label(self, language_profile: &LanguageProfile) -> SharedString {
        let key = match self {
            Self::Idle => "search_bar_placeholder",
            Self::Searching => "search_bar_searching",
            Self::Empty => "search_bar_no_matches",
            Self::SemanticUnavailable => "search_bar_semantic_unavailable",
            Self::PartialFailure => "search_bar_partial_failure",
            Self::Matches(count) => {
                return language_profile
                    .get_description("search_bar_match_count")
                    .unwrap()
                    .replace("{}", &count.to_string())
                    .into();
            }
        };

        language_profile.get_description(key).unwrap()
    }
}

pub async fn search(
    server_name: &SharedString,
    server_states: &ServerStates,
    databases: &Databases,
    block_ids: Vec<Uuid>,
    method: SupportedSearchMethod,
    system_configurations: SystemConfigurations,
    query_string: Option<String>,
    query_vector: Option<Vec<f32>>,
    top_n: usize,
) -> Result<Vec<SearchResult>> {
    let raw = route_search_blocks(
        &server_name,
        &server_states,
        &databases,
        system_configurations.clone(),
        method,
        block_ids,
        query_string,
        query_vector,
        top_n,
    )
    .await?;

    if raw.is_empty() {
        return Ok(Vec::new());
    }

    let ids = raw.iter().map(|r| r.block_id).collect::<HashSet<_>>();
    let blocks = route_read_blocks(
        &server_name,
        &server_states,
        &databases,
        &BlockQuery::ByIds(ids.into_iter().collect()),
        false,
        true,
        system_configurations,
    )
    .await?;

    let blocks: HashMap<_, _> = blocks.into_iter().map(|b| (b.id, b)).collect();
    let mut results = Vec::new();

    for raw in raw {
        let Some(block) = blocks.get(&raw.block_id) else {
            continue;
        };

        let Some(payload) = block.payloads.iter().find(|p| p.id == raw.payload_id) else {
            continue;
        };

        results.push(SearchResult {
            breadcrumb: get_breadcrumb(&server_name, block, &server_states.blocks),
            block: block.clone(),
            payload: payload.clone(),
            raw,
            server: server_name.clone(),
        });
    }

    Ok(results)
}

fn get_breadcrumb(server: &str, block: &Block, blocks: &HashMap<Uuid, Block>) -> SharedString {
    let mut titles = vec![block.get_title()];
    let mut seen = HashSet::from([block.id]);
    let mut parent = block.parent_id;

    while let Some(id) = parent {
        if !seen.insert(id) {
            titles.push("…".into());
            break;
        }

        let Some(ancestor) = blocks.get(&id) else {
            titles.push("…".into());
            break;
        };

        titles.push(ancestor.get_title());
        parent = ancestor.parent_id;
    }

    titles.push(server.to_owned());
    titles.reverse();
    titles.join(" › ").into()
}
