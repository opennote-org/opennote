use anyhow::Result;

use futures::future::join;
use uuid::Uuid;

use opennote_data::{
    Databases,
    database::enums::{BlockQuery, PayloadQuery},
};
use opennote_models::{block::Block, configurations::system::VectorDatabaseConfig};

/// Create num of empty notes
/// You can update them with the update function
pub async fn create_blocks(databases: &Databases, num_blocks: usize) -> Result<Vec<Block>> {
    databases.database.create_blocks(num_blocks).await
}

/// Read blocks from the database
pub async fn read_blocks(databases: &Databases, filter: &BlockQuery) -> Result<Vec<Block>> {
    databases.database.read_blocks(filter).await
}

pub async fn update_blocks(databases: &Databases, blocks: Vec<Block>) -> Result<()> {
    databases.database.update_blocks(blocks).await
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
