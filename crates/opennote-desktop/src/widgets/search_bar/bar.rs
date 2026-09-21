use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, Subscription, WeakEntity, Window,
    base::{h_flex, v_flex},
    component::{
        ActiveTheme, IndexPath,
        input::InputState,
        list::{ListDelegate, ListState},
        select::SelectState,
    },
    div, px,
};

use opennote_models::configurations::fields::search::SupportedSearchMethod;

use crate::{
    globals::{helpers::get_language_profile, states::helpers::get_states},
    widgets::{
        floating::create_float_palette,
        pane::Pane,
        search_bar::{
            search_results::SearchResultsList,
            subscriptions::{
                search_scope_labels, subscribe_search_query, subscribe_search_results,
                subscribe_search_scope,
            },
            view::{render_result_column, render_search_controls},
        },
    },
};

pub struct SearchBar {
    pub is_toggled: bool,
    pub keyword_search_results_list: Entity<ListState<SearchResultsList>>,
    pub semantic_search_results_list: Entity<ListState<SearchResultsList>>,

    pub query_input: Entity<InputState>,
    pub search_scope_state: Entity<SelectState<Vec<SharedString>>>,

    pub focus_handle: FocusHandle,

    pub _subscriptions: Vec<Subscription>,
    pub pane: WeakEntity<Pane>,
}

impl SearchBar {
    pub fn new(cx: &mut Context<Self>, window: &mut Window, pane: WeakEntity<Pane>) -> Self {
        let search_bar = cx.weak_entity();

        let keyword_search_results_list = cx.new(|cx| {
            ListState::new(
                SearchResultsList::new(search_bar.clone(), SupportedSearchMethod::Keyword),
                window,
                cx,
            )
        });
        let semantic_search_results_list = cx.new(|cx| {
            ListState::new(
                SearchResultsList::new(search_bar.clone(), SupportedSearchMethod::Semantic),
                window,
                cx,
            )
        });

        let placeholder = get_language_profile(cx).unwrap()["search_bar_placeholder"].clone();
        let query_input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));

        let search_scopes = search_scope_labels(cx);
        let search_scope_state = cx.new(|cx| {
            SelectState::new(
                search_scopes,
                Some(IndexPath::new(get_states(cx).get_search_scope_index())),
                window,
                cx,
            )
        });

        let subscriptions = vec![
            subscribe_search_results(cx, &keyword_search_results_list),
            subscribe_search_results(cx, &semantic_search_results_list),
            subscribe_search_query(cx, window, &query_input),
            subscribe_search_scope(cx, window, &search_scope_state),
        ];

        Self {
            is_toggled: false,
            keyword_search_results_list,
            semantic_search_results_list,
            query_input,
            search_scope_state,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
            pane,
        }
    }

    pub fn search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.query_input.read(cx).value();

        for list in [
            &self.keyword_search_results_list,
            &self.semantic_search_results_list,
        ] {
            list.update(cx, |list, cx| {
                list.set_selected_index(None, window, cx);
                list.delegate_mut()
                    .perform_search(&query, window, cx)
                    .detach();
            });
        }
    }

    pub fn get_input_field_focus_handle(&self, cx: &App) -> FocusHandle {
        self.query_input.focus_handle(cx)
    }
}

impl Focusable for SearchBar {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SearchBar {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        let profile = get_language_profile(cx).unwrap();
        let width = (window.viewport_size().width - px(48.)).min(px(960.));

        // Leave room for the input, column headings and outer window margins.
        let results_height = (window.viewport_size().height - px(200.)).max(px(120.));
        let columns = h_flex()
            .w_full()
            .items_start()
            .child(render_result_column(
                "search_bar_keyword_matches",
                &self.keyword_search_results_list,
                results_height,
                &profile,
                window,
                cx,
            ))
            .child(div().w_px().self_stretch().bg(cx.theme().border))
            .child(render_result_column(
                "search_bar_semantic_matches",
                &self.semantic_search_results_list,
                results_height,
                &profile,
                window,
                cx,
            ));

        create_float_palette(&self.focus_handle(cx), self.is_toggled).child(
            v_flex()
                .w(width)
                .rounded_xl()
                .overflow_hidden()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .text_color(cx.theme().popover_foreground)
                .shadow_2xl()
                .child(render_search_controls(self, cx))
                .child(columns),
        )
    }
}
