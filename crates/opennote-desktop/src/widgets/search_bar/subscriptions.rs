use std::str::FromStr;

use gpui::{Context, Entity, SharedString, Subscription};
use gpui_component::{
    list::ListState,
    select::{SelectEvent, SelectState},
};

use opennote_data::search::SearchScope;
use opennote_models::configurations::fields::search::SupportedSearchMethod;

use crate::{
    globals::{bootstrap::GlobalApplicationBootStrap, states::States},
    widgets::search_bar::{bar::SearchBar, search_results::SearchResultsList},
};

pub fn subscribe_search_method(
    cx: &mut Context<'_, SearchBar>,
    search_results_list_weak_entity: gpui::WeakEntity<ListState<SearchResultsList>>,
    search_method_state: &Entity<SelectState<Vec<SharedString>>>,
) -> Subscription {
    cx.subscribe(
        search_method_state,
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

            let new_search_method = SupportedSearchMethod::from_str(&new_search_method).unwrap();

            let bootstrap: &mut GlobalApplicationBootStrap = cx.global_mut();
            bootstrap.set_search_method(new_search_method);

            let _ = search_results_list_weak_entity.update(cx, |this, cx| {
                let delegate = this.delegate_mut();
                delegate.results.clear();
                cx.notify();
            });

            cx.notify();
        },
    )
}

pub fn subscribe_search_scope(
    cx: &mut Context<'_, SearchBar>,
    search_results_list_weak_entity_for_search_scope_state: gpui::WeakEntity<
        ListState<SearchResultsList>,
    >,
    search_scope_state: &Entity<SelectState<Vec<SharedString>>>,
) -> Subscription {
    cx.subscribe(
        search_scope_state,
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

            let _ =
                search_results_list_weak_entity_for_search_scope_state.update(cx, |this, cx| {
                    let delegate = this.delegate_mut();
                    delegate.results.clear();
                    cx.notify();
                });

            cx.notify();
        },
    )
}
