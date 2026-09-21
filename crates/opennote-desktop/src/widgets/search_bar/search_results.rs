use std::collections::{HashMap, HashSet};

use gpui_kit::{
    Context, InteractiveElement, ParentElement, SharedString, StatefulInteractiveElement, Styled,
    WeakEntity,
    component::{
        ActiveTheme, IndexPath,
        list::{ListDelegate, ListItem, ListState},
        tooltip::Tooltip,
        v_flex,
    },
    div,
};

use opennote_data::search::SearchScope;
use opennote_models::{
    block::Block, configurations::fields::search::SupportedSearchMethod, payload::Payload,
    search::RawSearchResult,
};

use crate::{
    globals::{
        bootstrap::GlobalApplicationBootStrap, helpers::get_language_profile, states::States,
    },
    widgets::{
        pane::helpers::open_block,
        search_bar::{
            bar::SearchBar,
            highlight::highlight_keyword_text,
            search::{SearchRequest, spawn_search},
        },
    },
};

pub struct SearchResult {
    pub block: Block,
    pub payload: Payload,
    pub raw: RawSearchResult,
    pub server: SharedString,
    pub breadcrumb: SharedString,
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
    pub fn label(self, language_profile: &HashMap<String, String>) -> SharedString {
        let key = match self {
            Self::Idle => "search_bar_placeholder",
            Self::Searching => "search_bar_searching",
            Self::Empty => "search_bar_no_matches",
            Self::SemanticUnavailable => "search_bar_semantic_unavailable",
            Self::PartialFailure => "search_bar_partial_failure",
            Self::Matches(count) => {
                return language_profile["search_bar_match_count"]
                    .replace("{}", &count.to_string())
                    .into();
            }
        };

        language_profile[key].clone().into()
    }
}

pub struct SearchResultsList {
    pub results: Vec<SearchResult>,
    pub active_query_id: usize,
    pub search_bar: WeakEntity<SearchBar>,
    pub selected_index: Option<IndexPath>,
    pub search_method: SupportedSearchMethod,
    pub status: SearchStatus,
    query: String,
}

impl SearchResultsList {
    pub fn new(search_bar: WeakEntity<SearchBar>, search_method: SupportedSearchMethod) -> Self {
        Self {
            results: Vec::new(),
            active_query_id: 0,
            search_bar,
            selected_index: None,
            search_method,
            status: SearchStatus::Idle,
            query: String::new(),
        }
    }

    pub(super) fn extend_results(&mut self, results: Vec<SearchResult>, top_n: usize) {
        self.results.extend(results);
        self.results.sort_by(|a, b| {
            b.raw
                .score
                .total_cmp(&a.raw.score)
                .then_with(|| a.server.cmp(&b.server))
                .then_with(|| a.raw.block_id.cmp(&b.raw.block_id))
                .then_with(|| a.raw.payload_id.cmp(&b.raw.payload_id))
        });
        let mut seen = HashSet::new();
        self.results.retain(|result| {
            seen.insert((
                result.server.clone(),
                result.raw.block_id,
                result.raw.payload_id,
            ))
        });
        self.results.truncate(top_n);
    }
}

impl ListDelegate for SearchResultsList {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &gpui_kit::App) -> usize {
        self.results.len()
    }

    fn render_empty(
        &mut self,
        _: &mut gpui_kit::Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl gpui_kit::IntoElement {
        let profile = get_language_profile(cx).unwrap();
        let message = match self.status {
            SearchStatus::Idle => {
                let key = match self.search_method {
                    SupportedSearchMethod::Keyword => "search_bar_keyword_hint",
                    SupportedSearchMethod::Semantic => "search_bar_semantic_hint",
                };
                SharedString::from(profile[key].clone())
            }
            _ => self.status.label(&profile),
        };
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .p_6()
            .text_sm()
            .text_center()
            .text_color(cx.theme().muted_foreground)
            .child(message)
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut gpui_kit::Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.results.get(ix.row).map(|result| {
            let texts = SharedString::from(result.payload.texts.clone());
            let passage = match self.search_method {
                SupportedSearchMethod::Keyword => {
                    highlight_keyword_text(texts.clone(), &self.query, cx)
                }
                SupportedSearchMethod::Semantic => gpui_kit::StyledText::new(texts.clone()),
            };
            let search_bar = self.search_bar.clone();
            let path = result.breadcrumb.clone();
            let block_id = result.block.id;

            ListItem::new(ix)
                .selected(Some(ix) == self.selected_index)
                .w_full()
                .flex_shrink_0()
                .py(gpui_kit::px(12.))
                .px_4()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .gap_1()
                        .child(
                            div()
                                .w_full()
                                .flex_shrink_0()
                                .text_sm()
                                .line_height(gpui_kit::px(20.))
                                .child(passage),
                        )
                        .child(
                            div()
                                .id(("breadcrumb", ix.row))
                                .w_full()
                                .flex_shrink_0()
                                .text_xs()
                                .line_height(gpui_kit::px(20.))
                                .text_color(cx.theme().muted_foreground)
                                .child(path.clone())
                                .tooltip(move |window, cx| {
                                    Tooltip::new(path.clone()).build(window, cx)
                                }),
                        ),
                )
                .on_click(cx.listener(move |_, _, window, cx| {
                    open_block(cx, window, block_id, Some(texts.clone()));

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
        _: &mut gpui_kit::Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(
        &mut self,
        _: bool,
        window: &mut gpui_kit::Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        let Some(result) = self.selected_index.and_then(|ix| self.results.get(ix.row)) else {
            return;
        };

        open_block(
            cx,
            window,
            result.block.id,
            Some(result.payload.texts.clone().into()),
        );

        let _ = self.search_bar.update(cx, |bar, cx| {
            bar.is_toggled = false;
            cx.notify();
        });
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut gpui_kit::Window,
        cx: &mut Context<ListState<Self>>,
    ) -> gpui_kit::Task<()> {
        self.query = query.to_owned();
        self.active_query_id += 1;
        let query_id = self.active_query_id;

        self.results.clear();
        self.selected_index = None;
        self.status = SearchStatus::Empty;
        cx.notify();

        if query.trim().is_empty() {
            self.status = SearchStatus::Idle;
            return gpui_kit::Task::ready(());
        }

        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let top_n = bootstrap.get_configurations().user.search.top_n;
        let states: &States = cx.global();

        let selected_block_id = states
            .get_active_pane(cx)
            .and_then(|pane| pane.read_with(cx, |pane, _| pane.selected_block_id).ok())
            .flatten();

        let (servers, block_ids) = match states.get_search_scope() {
            SearchScope::Document => match selected_block_id {
                Some(result) => {
                    let block_ids = vec![result];
                    (states.get_servers_by_block_ids(&block_ids), block_ids)
                }
                None => return gpui_kit::Task::ready(()),
            },
            SearchScope::Collection => {
                // Get the selected block id
                let block_id = match selected_block_id {
                    Some(result) => result,
                    None => return gpui_kit::Task::ready(()),
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

        if block_ids.is_empty() || servers.is_empty() {
            return gpui_kit::Task::ready(());
        }

        let request = SearchRequest {
            method: self.search_method,
            query: query.to_owned(),
            vector: None,
            top_n,
        };

        self.status = SearchStatus::Searching;

        spawn_search(query_id, request, servers, block_ids, cx)
    }
}
