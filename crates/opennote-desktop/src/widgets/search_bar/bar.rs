use std::str::FromStr;

use anyhow::Context as AnyhowContext;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, Subscription, div,
};
use gpui_component::{
    ActiveTheme, IndexPath, Sizable, StyledExt, h_flex,
    list::{List, ListState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};

use opennote_data::search::SearchScope;
use opennote_models::configurations::search::SupportedSearchMethod;

use crate::{
    globals::{
        bootstrap::{GlobalApplicationBootStrap, SEARCH_METHODS_ENUMS, SEARCH_SCOPES_ENUMS},
        helpers::get_language_profile,
        states::States,
    },
    widgets::{floating::create_float_palette, search_bar::search_results::SearchResultsList},
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

        let search_results_list = cx.new(|cx| {
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
        _subscriptions.push(cx.subscribe(
            &search_method_state,
            move |_this, _tree_state, event: &SelectEvent<Vec<SharedString>>, cx| {
                let new_search_method = match event {
                    SelectEvent::Confirm(value) => {
                        let Some(value) = value else {
                            return;
                        };
                        value
                    }
                };

                let new_search_method = new_search_method.to_owned();

                let new_search_method =
                    SupportedSearchMethod::from_str(&new_search_method).unwrap();

                let bootstrap: &mut GlobalApplicationBootStrap = cx.global_mut();
                bootstrap.set_search_method(new_search_method);

                let _ = search_results_list_weak_entity.update(cx, |this, cx| {
                    let delegate = this.delegate_mut();
                    delegate.results.clear();
                    cx.notify();
                });

                cx.notify();
            },
        ));

        // Update the search scope when the selected search scope changes
        _subscriptions.push(cx.subscribe(
            &search_scope_state,
            move |_this, _tree_state, event: &SelectEvent<Vec<SharedString>>, cx| {
                let new_search_scope = match event {
                    SelectEvent::Confirm(value) => {
                        let Some(value) = value else {
                            return;
                        };
                        value
                    }
                };

                let new_search_scope = SearchScope::from_str(&new_search_scope.to_owned()).unwrap();

                let states: &mut States = cx.global_mut();
                states.set_search_scope(new_search_scope);

                let _ = search_results_list_weak_entity_for_search_scope_state.update(
                    cx,
                    |this, cx| {
                        let delegate = this.delegate_mut();
                        delegate.results.clear();
                        cx.notify();
                    },
                );

                cx.notify();
            },
        ));

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
