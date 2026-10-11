use std::collections::HashMap;

use anyhow::Result;
use futures::future::try_join_all;
use gpui_kit::{App, SharedString};
use uuid::Uuid;

use opennote_core_logics::{
    helpers::run_async_code,
    payload::{PayloadContentParameters, build_payload, convert_string_to_payloads},
};
use opennote_data::Databases;
use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};
use opennote_models::{
    block::Block,
    configurations::{desktop::DesktopConfigurations, system::SystemConfigurations},
    query::BlockQuery,
};

use crate::globals::states::server_registry::ServerStates;
use crate::globals::{
    actions::route_helpers::route_read_blocks, bootstrap::helpers::get_bootstrap,
    states::helpers::get_states,
};

/// Get the full content of the specified block from the originated server.
///
/// The get method provided by the block is only able to get the content that it caches.
/// Since in the desktop app we treat each block as a reference,
/// each block won't reserve the full payloads in the cache.
/// Therefore, we will need to use this method to retreive the actual full content of a block.
pub fn get_block_content(block_id: &Uuid, cx: &mut App) -> Result<String> {
    let block_ids = vec![*block_id];

    let states = get_states(cx);
    let (server_name, server_states) = states.get_servers_by_block_ids(&block_ids).remove(0);

    let bootstrap = get_bootstrap(cx);

    let block = run_async_code(async {
        let system_configurations = bootstrap.0.configurations.lock().await.system.clone();
        route_read_blocks(
            &server_name,
            &server_states,
            &bootstrap.0.databases,
            &BlockQuery::ByIds(block_ids),
            false,
            true,
            system_configurations,
        )
        .await
        .unwrap()
        .remove(0)
    });

    Ok(block.get_text_content())
}

/// Default block title: the title of the block
pub async fn build_block(
    parent_block_id: Option<Uuid>,
    default_block_title: String,
    embedders: &EmbedderEntry,
    content: Option<String>,
    text_chunk_size: Option<usize>,
) -> Result<Block, anyhow::Error> {
    let mut block = Block::new(parent_block_id, Vec::new());

    let payloads = match content {
        Some(content) => convert_string_to_payloads(
            block.id,
            text_chunk_size,
            content,
            Some(default_block_title),
        )?,
        None => vec![build_payload(
            block.id,
            PayloadContentParameters {
                title: Some(default_block_title.to_string()),
                ..Default::default()
            },
        )?],
    };

    let vectorized_payloads = send_vectorization(payloads, &embedders).await?;

    block.payloads = vectorized_payloads;

    Ok(block)
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
