use std::collections::HashMap;

use gpui_kit::{BorrowAppContext, SharedString, component::Selectable};

use crate::{
    globals::states::{States, server_registry::ServerStates},
    libs::tabs::{tab::Tab, tab_bar::TabBar},
};

pub fn create_sidebar_tabbar(
    active_server: SharedString,
    servers: &HashMap<SharedString, ServerStates>,
) -> TabBar {
    TabBar::new("sidebar-tabs").children(servers.keys().map(|server_name| {
        // Cheap clone
        let server_name = server_name.clone();
        // Has the server been selected?
        let has_selected: bool = active_server == server_name;

        Tab::new()
            .label(server_name.clone())
            .selected(has_selected)
            .on_click(move |event: &gpui_kit::ClickEvent, window, cx| {
                if !event.is_right_click() {
                    let window_id = window.window_handle().window_id();
                    cx.update_global::<States, ()>(|states, cx| {
                        states.set_active_server(window_id, server_name.clone());
                        states.refresh_blocks_list(cx);
                    });
                }
            })
    }))
}
