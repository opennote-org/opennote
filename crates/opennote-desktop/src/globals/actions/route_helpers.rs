use anyhow::Result;
use reqwest::Client;
use uuid::Uuid;

use opennote_core_logics::{
    block::{create_blocks, delete_blocks, read_blocks, update_blocks},
    search::{search_by_keyword, search_by_semantics},
};
use opennote_data::Databases;
use opennote_models::{
    block::Block,
    configurations::{
        fields::{VectorDatabaseConfig, search::SupportedSearchMethod},
        system::SystemConfigurations,
    },
    constants::LOCAL_SERVER_NAME,
    query::BlockQuery,
    search::RawSearchResult,
};
use opennote_server::{
    create_remote_server_blocks, delete_remote_server_blocks, read_remote_server_blocks,
    search_remote_server_blocks, update_remote_server_blocks,
};

use crate::globals::states::server_registry::ServerStates;

pub async fn route_create_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    system_configuraitons: SystemConfigurations,
    blocks: Vec<Block>,
) -> Result<Vec<Block>> {
    if server_name == LOCAL_SERVER_NAME {
        create_blocks(vector_database_config, databases, blocks).await
    } else {
        create_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            blocks,
            &server_states.shared_key,
            system_configuraitons,
        )
        .await
    }
}

pub async fn route_delete_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    system_configuraitons: SystemConfigurations,
    block_ids: Vec<Uuid>,
) -> Result<()> {
    if server_name == LOCAL_SERVER_NAME {
        delete_blocks(databases, vector_database_config, block_ids).await
    } else {
        delete_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            block_ids,
            &server_states.shared_key,
            system_configuraitons,
        )
        .await
    }
}

pub async fn route_read_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    filter: &BlockQuery,
    has_vector: bool,
    has_payload: bool,
    system_configuraitons: SystemConfigurations,
) -> Result<Vec<Block>> {
    if server_name == LOCAL_SERVER_NAME {
        read_blocks(databases, filter, has_vector, has_payload).await
    } else {
        read_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            &server_states.shared_key,
            filter,
            has_vector,
            has_payload,
            system_configuraitons,
        )
        .await
    }
}

pub async fn route_update_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    vector_database_config: &VectorDatabaseConfig,
    system_configuraitons: SystemConfigurations,
    blocks: Vec<Block>,
) -> Result<()> {
    if server_name == LOCAL_SERVER_NAME {
        update_blocks(vector_database_config, databases, blocks).await
    } else {
        update_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            blocks,
            &server_states.shared_key,
            system_configuraitons,
        )
        .await
    }
}

pub async fn route_search_blocks(
    server_name: &str,
    server_states: &ServerStates,
    databases: &Databases,
    system_configuraitons: SystemConfigurations,
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
        search_remote_server_blocks(
            &Client::new(),
            &server_states.connection_string,
            &server_states.password,
            search_method,
            block_ids,
            query,
            query_vector,
            top_n,
            &server_states.shared_key,
            system_configuraitons,
        )
        .await
    }
}
