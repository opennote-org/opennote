use gpui_kit::{
    Context, Entity, SharedString, Subscription, Window,
    component::{
        input::{InputEvent, InputState},
        list::ListState,
        select::{SelectEvent, SelectState},
    },
};

use crate::{
    globals::{
        bootstrap::SEARCH_SCOPES_ENUMS, helpers::get_language_profile,
        states::helpers::get_states_mut,
    },
    widgets::search_bar::{bar::SearchBar, search_results::SearchResultsList},
};

pub fn subscribe_search_results(
    cx: &mut Context<SearchBar>,
    list: &Entity<ListState<SearchResultsList>>,
) -> Subscription {
    cx.observe(list, |_, _, cx| cx.notify())
}

pub fn subscribe_search_query(
    cx: &mut Context<SearchBar>,
    window: &mut Window,
    input: &Entity<InputState>,
) -> Subscription {
    cx.subscribe_in(input, window, |bar, _, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            bar.search(window, cx);
        }
    })
}

pub fn search_scope_labels(cx: &gpui_kit::App) -> Vec<SharedString> {
    let profile = get_language_profile(cx).unwrap();

    SEARCH_SCOPES_ENUMS
        .iter()
        .map(|scope| {
            let key = match scope {
                opennote_data::search::SearchScope::Document => "search_bar_scope_document",
                opennote_data::search::SearchScope::Collection => "search_bar_scope_collection",
                opennote_data::search::SearchScope::Userspace => "search_bar_scope_userspace",
            };
            profile[key].clone().into()
        })
        .collect()
}

pub fn subscribe_search_scope(
    cx: &mut Context<SearchBar>,
    window: &mut Window,
    scope: &Entity<SelectState<Vec<SharedString>>>,
) -> Subscription {
    let labels = search_scope_labels(cx);

    cx.subscribe_in(
        scope,
        window,
        move |bar, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };

            let Some(index) = labels.iter().position(|label| label == value) else {
                return;
            };

            get_states_mut(cx).set_search_scope(SEARCH_SCOPES_ENUMS[index]);
            bar.search(window, cx);
        },
    )
}
