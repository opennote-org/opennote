use actix_web::web::Bytes;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_encrypt::{
    EncryptedMessage, serialize::impls::BincodeSerializer, shared_key::SharedKey,
    traits::SerdeEncryptSharedKey,
};
use uuid::Uuid;

use crate::{
    block::Block,
    configurations::{fields::search::SupportedSearchMethod, system::SystemConfigurations},
    query::BlockQuery,
    traits::GetSystemConfigurations,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRequest<T> {
    pub payload: T,
}

impl<T> SerdeEncryptSharedKey for BaseRequest<T> {
    type S = BincodeSerializer<Self>;
}

pub fn create_request<T: Serialize>(
    payload: T,
    shared_key: &SharedKey,
) -> Result<EncryptedMessage, serde_encrypt::Error> {
    let request = BaseRequest { payload };
    request.encrypt(shared_key)
}

pub fn decrypt_request<G: DeserializeOwned>(
    request: Bytes,
    shared_key: &SharedKey,
) -> Result<G, serde_encrypt::Error> {
    let encrypted_message: EncryptedMessage = EncryptedMessage::deserialize(request.to_vec())?;
    let base_request: BaseRequest<G> = BaseRequest::decrypt_owned(&encrypted_message, shared_key)?;
    Ok(base_request.payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub block_query: BlockQuery,
    pub has_vector: bool,
    pub has_payload: bool,
}

impl GetSystemConfigurations for ReadBlocksInWorkspaceRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub blocks: Vec<Block>,
}

impl GetSystemConfigurations for CreateBlocksInWorkspaceRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub block_ids: Vec<Uuid>,
}

impl GetSystemConfigurations for DeleteBlocksInWorkspaceRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub blocks: Vec<Block>,
}

impl GetSystemConfigurations for UpdateBlocksInWorkspaceRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub search_method: SupportedSearchMethod,
    pub block_ids: Vec<Uuid>,
    pub query: Option<String>,
    pub query_vector: Option<Vec<f32>>,
    pub top_n: usize,
}

impl GetSystemConfigurations for SearchBlocksInWorkspaceRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestReindexBlocksRequest {
    pub system_configurations: SystemConfigurations,
}

impl GetSystemConfigurations for RequestReindexBlocksRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}

/// After done a `ReindexBlocksRequest`,
/// Use this to send the embedded blocks back to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendReindexedBlocksRequest {
    pub system_configurations: SystemConfigurations,
    /// Rindexed / embedded blocks
    pub blocks: Vec<Block>,
}

impl GetSystemConfigurations for SendReindexedBlocksRequest {
    fn get_system_configurations(&self) -> &SystemConfigurations {
        &self.system_configurations
    }
}
