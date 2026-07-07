use std::collections::HashMap;

use gpui::{BorrowAppContext, SharedString};
use gpui_component::Selectable;

use crate::{
    globals::states::{ServerStates, States},
    libs::tabs::{tab::Tab, tab_bar::TabBar},
};

pub fn create_sidebar_tabbar(
    active_server: SharedString,
    servers: &HashMap<SharedString, ServerStates>,
) -> TabBar {
    let tabs =
        TabBar::new("sidebar-tabs").children(servers.iter().map(|(server_name, _server_state)| {
            // Cheap clone
            let server_name = server_name.clone();
            // Has the server been selected?
            let has_selected: bool = active_server == server_name;

            let tab = Tab::new()
                .label(server_name.clone())
                .selected(has_selected)
                .on_click(move |event: &gpui::ClickEvent, _window, cx| {
                    if !event.is_right_click() {
                        let _ = cx.update_global::<States, ()>(|states, cx| {
                            states.set_active_remote_server(server_name.clone());
                            states.refresh_blocks_list(cx);
                        });
                    }
                });

            tab
        }));
    tabs
}
