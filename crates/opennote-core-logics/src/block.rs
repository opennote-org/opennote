use anyhow::Result;

use futures::future::join;
use uuid::Uuid;

use opennote_data::{
    Databases,
    database::enums::{BlockQuery, PayloadQuery},
};
use opennote_models::{
    block::Block, configurations::system::VectorDatabaseConfig, payload::Payload,
};

pub async fn create_blocks(
    vector_database_config: &VectorDatabaseConfig,
    databases: &Databases,
    blocks: Vec<Block>,
) -> Result<Vec<Block>> {
    let payloads: Vec<Payload> = blocks
        .iter()
        .flat_map(|block| block.payloads.clone())
        .collect();

    let (_, blocks) = join(
        databases
            .vector_database
            .create_entries(&vector_database_config.index, payloads),
        databases.database.create_blocks(blocks),
    )
    .await;

    Ok(blocks?)
}

/// Read blocks from the database
pub async fn read_blocks(databases: &Databases, filter: &BlockQuery) -> Result<Vec<Block>> {
    databases.database.read_blocks(filter).await
}

pub async fn update_blocks(
    vector_database_config: &VectorDatabaseConfig,
    databases: &Databases,
    blocks: Vec<Block>,
) -> Result<()> {
    let mut payload_ids = Vec::new();
    let mut payloads = Vec::new();

    for block in blocks.iter() {
        payload_ids.push(block.id);
        payloads.extend(block.payloads.clone());
    }

    databases
        .vector_database
        .delete_entries(vector_database_config, &payload_ids)
        .await?;

    let _ = join(
        databases
            .vector_database
            .create_entries(&vector_database_config.index, payloads),
        databases.database.update_blocks(blocks),
    )
    .await;

    Ok(())
}

pub async fn delete_blocks(
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    block_ids: Vec<Uuid>,
) -> Result<()> {
    let payloads = databases
        .database
        .delete_payloads(&PayloadQuery::ByBlockIds(block_ids.clone()))
        .await?;

    let (delete_blocks_result, delete_entries_results) = join(
        databases.database.delete_blocks(block_ids),
        databases.vector_database.delete_entries(
            vector_database_config,
            &payloads.into_iter().map(|item| item.id).collect(),
        ),
    )
    .await;

    delete_blocks_result?;
    delete_entries_results?;

    Ok(())
}

/// Create new payloads for blocks
/// However, this function does not check if the payloads exist already
pub async fn create_payloads(
    vector_database_config: &VectorDatabaseConfig,
    databases: &Databases,
    payloads: Vec<Payload>,
) -> Result<()> {
    join(
        databases.database.create_payloads(payloads.clone()),
        databases
            .vector_database
            .create_entries(&vector_database_config.index, payloads),
    )
    .await;

    Ok(())
}

/// Reover all blocks from the relational database to the vector database
pub async fn recover(databases: &Databases, index: &str, dimensions: usize) -> Result<()> {
    let payloads = databases.database.read_payloads(&PayloadQuery::All).await?;

    databases
        .vector_database
        .reset_index(index, dimensions)
        .await?;

    databases
        .vector_database
        .create_entries(index, payloads)
        .await?;

    Ok(())
}
