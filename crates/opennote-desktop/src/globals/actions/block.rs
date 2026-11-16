use std::collections::HashMap;

use anyhow::{Result, anyhow};
use futures::future::try_join_all;
use gpui_kit::{App, SharedString, Window};
use uuid::Uuid;

use opennote_core_logics::helpers::run_async_code;
use opennote_data::Databases;
use opennote_models::{
    block::Block, configurations::system::SystemConfigurations, constants::LOCAL_SERVER_NAME,
    query::BlockQuery,
};

use crate::globals::{
    actions::{assets::download_block_assets, route_helpers::route_read_blocks},
    bootstrap::helpers::get_bootstrap,
    states::{helpers::get_states, server_registry::ServerStates},
};

/// Get the full content of the specified block from the originated server.
///
/// The get method provided by the block is only able to get the content that it caches.
/// Since in the desktop app we treat each block as a reference,
/// each block won't reserve the full payloads in the cache.
/// Therefore, we will need to use this method to retreive the actual full content of a block.
pub fn get_block_content(block_id: &Uuid, cx: &mut App, window: &mut Window) -> Result<String> {
    let block_ids = vec![*block_id];

    let states = get_states(cx);
    let (server_name, server_states) = match states
        .get_servers_by_block_ids(&block_ids)
        .into_iter()
        .next()
    {
        Some(server) => server,
        None => return Err(anyhow!("No server found for block {block_id}")),
    };

    let bootstrap = get_bootstrap(cx);
    let assets_storage = bootstrap.0.assets_storage.clone();

    let (block, system_configurations) = run_async_code(async {
        let system_configurations = bootstrap.0.configurations.lock().await.system.clone();

        let blocks = route_read_blocks(
            &server_name,
            &server_states,
            &bootstrap.0.databases,
            &BlockQuery::ByIds(block_ids),
            false,
            true,
            system_configurations.clone(),
        )
        .await?;

        let block = match blocks.into_iter().next() {
            Some(block) => block,
            None => return Err(anyhow!("Block {block_id} not found")),
        };

        Ok((block, system_configurations))
    })?;

    let texts = block.get_text_content();
    if server_name == LOCAL_SERVER_NAME || block.get_assets_references().is_empty() {
        return Ok(texts);
    }

    download_block_assets(
        cx,
        window,
        server_name,
        server_states,
        assets_storage,
        block,
        system_configurations,
    )?;

    Ok(texts)
}

/// Use this function to get blocks from servers.
/// It supports filtering by server attributes.
///
/// `has_payload` set to false will only return the title in each block.
pub async fn get_blocks_from_servers(
    database_filter: BlockQuery,
    server_filter: impl Fn(&&SharedString, &&ServerStates) -> bool,
    servers: HashMap<SharedString, ServerStates>,
    databases: Databases,
    system_configurations: SystemConfigurations,
    has_vector: bool,
    has_payload: bool,
) -> Result<Vec<Block>> {
    Ok(try_join_all(
        servers
            .iter()
            .filter(|(server_name, server_states)| server_filter(server_name, server_states))
            .map(|(server_name, server_states)| {
                route_read_blocks(
                    server_name,
                    server_states,
                    &databases,
                    &database_filter,
                    has_vector,
                    has_payload,
                    system_configurations.clone(),
                )
            }),
    )
    .await?
    .into_iter()
    .flatten()
    .collect())
}
