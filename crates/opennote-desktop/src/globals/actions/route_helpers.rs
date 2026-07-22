use anyhow::Result;
use reqwest::Client;
use uuid::Uuid;

use opennote_core_logics::block::{create_blocks, delete_blocks, read_blocks, update_blocks};
use opennote_core_logics::search::{search_by_keyword, search_by_semantics};
use opennote_data::{Databases, search::models::RawSearchResult};
use opennote_models::{
    block::Block,
    configurations::{search::SupportedSearchMethod, fields::VectorDatabaseConfig},
    constants::LOCAL_SERVER_NAME,
    query::BlockQuery,
};
use opennote_server::{
    create_remote_server_blocks, delete_remote_server_blocks, read_remote_server_blocks,
    search_remote_server_blocks, update_remote_server_blocks,
};

use crate::globals::states::ServerStates;

pub async fn route_create_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    blocks: Vec<Block>,
) -> Result<Vec<Block>> {
    if server_name == LOCAL_SERVER_NAME {
        create_blocks(vector_database_config, databases, blocks).await
    } else {
        match create_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            blocks,
            &server_states.shared_key,
        )
        .await
        {
            Ok(blocks) => Ok(blocks),
            Err(_) => Ok(Vec::new()),
        }
    }
}

pub async fn route_delete_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    block_ids: Vec<Uuid>,
) -> Result<()> {
    if server_name == LOCAL_SERVER_NAME {
        delete_blocks(databases, vector_database_config, block_ids).await
    } else {
        match delete_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            block_ids,
            &server_states.shared_key,
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        }
    }
}

pub async fn route_read_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    filter: &BlockQuery,
    has_vector: bool,
    has_payload: bool,
) -> Result<Vec<Block>> {
    if server_name == LOCAL_SERVER_NAME {
        read_blocks(databases, filter, has_vector, has_payload).await
    } else {
        match read_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            &server_states.shared_key,
            filter,
            has_vector,
            has_payload,
        )
        .await
        {
            Ok(results) => Ok(results),
            Err(_) => Ok(Vec::new()),
        }
    }
}

pub async fn route_update_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    blocks: Vec<Block>,
) -> Result<()> {
    if server_name == LOCAL_SERVER_NAME {
        update_blocks(vector_database_config, databases, blocks).await
    } else {
        match update_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            blocks,
            &server_states.shared_key,
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        }
    }
}

pub async fn route_search_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    search_method: SupportedSearchMethod,
    block_ids: Vec<Uuid>,
    query: Option<String>,
    query_vector: Option<Vec<f32>>,
    top_n: usize,
) -> Result<Vec<RawSearchResult>> {
    if server_name == LOCAL_SERVER_NAME {
        match search_method {
            SupportedSearchMethod::Keyword => {
                // Early return for missing query value
                let query = query
                    .ok_or_else(|| anyhow::anyhow!("Query string required for keyword search"))?;
                search_by_keyword(databases, block_ids, &query, top_n).await
            }
            SupportedSearchMethod::Semantic => {
                // Early return for missing query value
                let query_vector = query_vector
                    .ok_or_else(|| anyhow::anyhow!("Query vector required for semantic search"))?;
                search_by_semantics(databases, block_ids, &query_vector, top_n).await
            }
        }
    } else {
        // Missing value check now is relied on the remote server
        match search_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            search_method,
            block_ids,
            query,
            query_vector,
            top_n,
            &server_states.shared_key,
        )
        .await
        {
            Ok(results) => Ok(results),
            Err(_) => Ok(Vec::new()),
        }
    }
}
