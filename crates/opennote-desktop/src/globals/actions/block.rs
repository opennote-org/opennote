use anyhow::Result;
use gpui::App;
use uuid::Uuid;

use opennote_core_logics::helpers::run_async_code;
use opennote_models::query::BlockQuery;

use crate::globals::{
    actions::route_helpers::route_read_blocks, bootstrap::GlobalApplicationBootStrap,
    states::States,
};

pub fn get_block_content(block_id: &Uuid, cx: &mut App) -> Result<String> {
    let block_ids = vec![*block_id];

    let states: &States = cx.global();
    let (server_name, server_states) = states.get_servers_by_block_ids(&block_ids).remove(0);

    let bootstrap: &GlobalApplicationBootStrap = cx.global();

    let block = run_async_code(async {
        route_read_blocks(
            &server_name,
            &server_states,
            &bootstrap.0.databases,
            &BlockQuery::ByIds(block_ids),
            false,
            true,
        )
        .await
        .unwrap()
        .remove(0)
    });

    Ok(block.get_text_content())
}
