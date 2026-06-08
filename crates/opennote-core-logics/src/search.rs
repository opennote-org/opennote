use anyhow::Result;
use uuid::Uuid;

use opennote_data::{Databases, database::enums::BlockQuery, search::models::RawSearchResult};

use crate::block::read_blocks;

pub async fn search_by_keyword(
    databases: &Databases,
    block_ids: Vec<Uuid>,
    query: &str,
    top_n: usize,
) -> Result<Vec<RawSearchResult>> {
    let payload_ids = get_payload_ids_by_block_ids(databases, block_ids).await?;

    // search
    databases
        .vector_database
        .search_documents(&databases.database, &payload_ids, query, top_n)
        .await
}

pub async fn search_by_semantics(
    databases: &Databases,
    block_ids: Vec<Uuid>,
    query: &[f32],
    top_n: usize,
) -> Result<Vec<RawSearchResult>> {
    let payload_ids = get_payload_ids_by_block_ids(databases, block_ids).await?;

    // search
    databases
        .vector_database
        .search_documents_semantically(&payload_ids, query, top_n)
        .await
}

async fn get_payload_ids_by_block_ids(
    databases: &Databases,
    block_ids: Vec<Uuid>,
) -> Result<Vec<Uuid>> {
    // get payloads
    let blocks = read_blocks(databases, &BlockQuery::ByIds(block_ids)).await?;

    Ok(blocks
        .iter()
        .flat_map(|item| item.payloads.iter().map(|item| item.id))
        .collect())
}
