use anyhow::Context as AnyhowContext;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, Subscription, div,
};
use gpui_component::{
    ActiveTheme, IndexPath, Sizable, StyledExt, h_flex,
    list::{List, ListState},
    select::{Select, SelectState},
    v_flex,
};

use crate::{
    globals::{
        bootstrap::{GlobalApplicationBootStrap, SEARCH_METHODS_ENUMS, SEARCH_SCOPES_ENUMS},
        helpers::get_language_profile,
        states::States,
    },
    widgets::{
        floating::create_float_palette,
        search_bar::{
            observations::observe_search_result_list,
            search_results::SearchResultsList,
            subscriptions::{subscribe_search_method, subscribe_search_scope},
        },
    },
};

/// Select commands to execute
pub struct SearchBar {
    pub is_toggled: bool,
    pub search_results_list: Entity<ListState<SearchResultsList>>,
    pub search_method_state: Entity<SelectState<Vec<SharedString>>>,
    pub search_scope_state: Entity<SelectState<Vec<SharedString>>>,

    pub focus_handle: FocusHandle,
    pub _subscriptions: Vec<Subscription>,
}

impl SearchBar {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        let mut _subscriptions = Vec::new();
        let search_bar_weak_entity = cx.weak_entity();

        // SelectState requires selecting methods based on index
        let search_methods: Vec<SharedString> = SEARCH_METHODS_ENUMS
            .into_iter()
            .map(|item| item.to_string().into())
            .collect();

        // SelectState requires selecting scopes based on index
        let search_scopes: Vec<SharedString> = SEARCH_SCOPES_ENUMS
            .into_iter()
            .map(|item| item.to_string().into())
            .collect();

        let search_results_list: Entity<ListState<SearchResultsList>> = cx.new(|cx| {
            ListState::new(SearchResultsList::new(search_bar_weak_entity), window, cx)
                .searchable(true)
        });

        let search_results_list_weak_entity = search_results_list.downgrade();
        let search_results_list_weak_entity_for_search_scope_state =
            search_results_list_weak_entity.clone();

        let search_method_state = cx.new(|cx| {
            let bootstrap: &GlobalApplicationBootStrap = cx.global();
            let selected_index: usize = bootstrap.get_search_method_index();

            SelectState::new(
                search_methods,
                Some(IndexPath::new(selected_index)),
                window,
                cx,
            )
        });

        let search_scope_state = cx.new(|cx| {
            let states: &States = cx.global();
            let selected_index = states.get_search_scope_index();

            SelectState::new(
                search_scopes,
                Some(IndexPath::new(selected_index)),
                window,
                cx,
            )
        });

        // Update the search method when the selected search method changes
        _subscriptions.push(subscribe_search_method(
            cx,
            search_results_list_weak_entity.clone(),
            &search_method_state,
        ));

        // Update the search scope when the selected search scope changes
        _subscriptions.push(subscribe_search_scope(
            cx,
            search_results_list_weak_entity_for_search_scope_state,
            &search_scope_state,
        ));

        // Observe the changes in the search results list.
        // We need to do this to update the final search results.
        _subscriptions.push(observe_search_result_list(cx, &search_results_list));

        Self {
            is_toggled: false,
            focus_handle: cx.focus_handle(),
            search_results_list,
            search_method_state,
            search_scope_state,
            _subscriptions,
        }
    }

    pub fn get_input_field_focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.search_results_list.focus_handle(cx)
    }
}

impl Focusable for SearchBar {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SearchBar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global())
            .context("Getting language profile failed")
            .unwrap();

        create_float_palette(&self.focus_handle(cx), self.is_toggled).child(
            h_flex()
                .flex_shrink()
                .items_start()
                .gap_2()
                .child(div().v_flex().gap_2().children([
                    Select::new(&self.search_method_state).w_40().small(),
                    Select::new(&self.search_scope_state).w_40().small(),
                ]))
                .child(
                    v_flex().child(
                        List::new(&self.search_results_list)
                            .search_placeholder(&language_profile["search_bar_placeholder"])
                            .bg(cx.theme().accent)
                            .shadow_2xl()
                            .w_128()
                            .h_128(),
                    ),
                ),
        )
    }
}
