use gpui_kit::{Context, Entity, SharedString, Window};

use crate::{
    globals::states::helpers::get_states,
    widgets::pane::Pane,
    windows::{helpers::format_window_title, workspace::Workspace},
};

pub fn construct_workspace_window_title(
    pane: Entity<Pane>,
    window: &mut Window,
    cx: &mut Context<'_, Workspace>,
) -> SharedString {
    // recompute the window title
    let states = get_states(cx);

    let pane = pane.read(cx);

    // TODO: Get the selected_block_id from the pane instead, not the editor.
    // Maybe add an annotation for standardized access
    let document_name = match &pane.selected_block_id {
        Some(block_id) => Some(states.get_block(block_id).unwrap().get_title()),
        None => None,
    };

    let server_name = match pane.selected_block_id {
        Some(block_id) => Some(
            states.get_servers_by_block_ids(&vec![block_id])[0]
                .0
                .to_string(),
        ),
        None => None,
    };

    let server_name = match server_name {
        Some(result) => result,
        None => states
            .get_active_server(window.window_handle().window_id())
            .0
            .to_string(),
    };

    format_window_title(None, Some(&server_name), document_name.as_deref())
}
