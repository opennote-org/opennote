use std::collections::HashMap;

use gpui_kit::{
    App, Context, Div, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    SharedString, StatefulInteractiveElement, Styled, Window,
    component::{
        ActiveTheme, Icon, IconName, IndexPath, Sizable, h_flex,
        input::Input,
        list::{ListDelegate, ListState},
        select::Select,
        v_flex,
    },
    div,
};

use crate::widgets::search_bar::{
    bar::SearchBar,
    search_results::{SearchResultsList, SearchStatus},
};

pub fn render_search_controls(bar: &SearchBar, cx: &App) -> Div {
    h_flex()
        .w_full()
        .items_center()
        .gap_3()
        .p_4()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div().flex_1().min_w_0().child(
                Input::new(&bar.query_input)
                    .w_full()
                    .cleanable(true)
                    .prefix(Icon::new(IconName::Search).text_color(cx.theme().muted_foreground)),
            ),
        )
        .child(
            div()
                .flex_shrink_0()
                .child(Select::new(&bar.search_scope_state).w_40().small()),
        )
}

pub fn render_result_column(
    title: &'static str,
    list: &Entity<ListState<SearchResultsList>>,
    height: Pixels,
    profile: &HashMap<String, String>,
    window: &mut Window,
    cx: &mut Context<SearchBar>,
) -> Div {
    let status = match list.read(cx).delegate().status {
        SearchStatus::Idle | SearchStatus::Empty => SharedString::default(),
        status => status.label(profile),
    };

    let content = list.update(cx, |state, cx| {
        render_results(state.delegate_mut(), window, cx)
    });

    v_flex()
        .flex_1()
        .min_w_0()
        .child(render_column_header(
            profile[title].clone().into(),
            status,
            cx,
        ))
        .child(
            div()
                .id(title)
                .w_full()
                .h(height)
                .overflow_y_scroll()
                .child(content),
        )
}

pub fn render_column_header(title: SharedString, status: SharedString, cx: &App) -> Div {
    h_flex()
        .w_full()
        .justify_between()
        .gap_2()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_sm()
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_xs()
                .truncate()
                .text_color(cx.theme().muted_foreground)
                .child(status),
        )
}

pub fn render_results(
    delegate: &mut SearchResultsList,
    window: &mut Window,
    cx: &mut Context<ListState<SearchResultsList>>,
) -> gpui_kit::AnyElement {
    if delegate.results.is_empty() {
        return delegate.render_empty(window, cx).into_any_element();
    }

    // Full passages need variable-height rows instead of List's fixed-height virtualization.
    let rows: Vec<_> = (0..delegate.results.len())
        .filter_map(|row| delegate.render_item(IndexPath::new(row), window, cx))
        .collect();

    v_flex().w_full().children(rows).into_any_element()
}
