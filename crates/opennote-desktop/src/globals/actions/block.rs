use anyhow::Result;
use gpui_kit::App;
use opennote_models::block::Block;
use uuid::Uuid;

use opennote_core_logics::helpers::run_async_code;
use opennote_core_logics::payload::{
    PayloadContentParameters, build_payload, convert_string_to_payloads,
};
use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};
use opennote_models::query::BlockQuery;

use crate::globals::{
    actions::route_helpers::route_read_blocks, bootstrap::GlobalApplicationBootStrap,
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
