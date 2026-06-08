use anyhow::Context as AnyhowContext;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, ParentElement,
    Render, SharedString, Styled, Subscription, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, IndexPath, Sizable, h_flex,
    list::{List, ListState},
    select::{Select, SelectState},
    v_flex,
};

use crate::{
    globals::{
        bootstrap::{GlobalApplicationBootStrap, SEARCH_METHODS_ENUMS},
        helpers::get_language_profile,
    },
    widgets::search_bar::search_results::SearchResultsList,
};

/// Select commands to execute
pub struct SearchBar {
    pub is_toggled: bool,
    pub search_results_list: Entity<ListState<SearchResultsList>>,
    pub search_method_state: Entity<SelectState<Vec<SharedString>>>,

    pub focus_handle: FocusHandle,
    pub _subscriptions: Vec<Subscription>,
}

impl SearchBar {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Self {
        let mut _subscriptions = Vec::new();

        let bootstrap: &GlobalApplicationBootStrap = cx.global();

        // SelectState requires selecting methods based on index
        let search_methods: Vec<SharedString> = SEARCH_METHODS_ENUMS
            .into_iter()
            .map(|item| item.to_string().into())
            .collect();

        let selected_index: usize = bootstrap.get_search_method();

        let search_method_state = cx.new(|cx| {
            SelectState::new(
                search_methods,
                Some(IndexPath::new(selected_index)),
                window,
                cx,
            )
        });

        _subscriptions.push(cx.observe(&search_method_state, |this, _tree_state, cx| {
            
        }));

        Self {
            is_toggled: false,
            search_results_list: cx
                .new(|cx| ListState::new(SearchResultsList::new(), window, cx).searchable(true)),
            focus_handle: cx.focus_handle(),
            search_method_state,
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
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let language_profile = get_language_profile(cx.global(), cx.global())
            .context("Getting language profile failed")
            .unwrap();

        div()
            .track_focus(&self.focus_handle(cx))
            .absolute()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .when(self.is_toggled, |this| this.visible())
            .when(!self.is_toggled, |this| this.invisible())
            .child(
                h_flex()
                    .flex_shrink()
                    .items_start()
                    .gap_2()
                    .child(Select::new(&self.search_method_state).w_40().small())
                    .child(
                        v_flex().child(
                            List::new(&self.search_results_list)
                                .search_placeholder(language_profile.search_bar_placeholder)
                                .bg(cx.theme().accent)
                                .shadow_2xl()
                                .w_128()
                                .h_128(),
                        ),
                    ),
            )
        // When changes had been detected, start full text match
    }
}
