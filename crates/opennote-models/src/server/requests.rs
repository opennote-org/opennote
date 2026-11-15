use actix_web::web::Bytes;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_encrypt::{
    EncryptedMessage, serialize::impls::BincodeSerializer, shared_key::SharedKey,
    traits::SerdeEncryptSharedKey,
};
use uuid::Uuid;

use opennote_derive::GetSystemConfigurations;

use crate::{
    block::Block,
    configurations::{fields::search::SupportedSearchMethod, system::SystemConfigurations},
    query::BlockQuery,
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

#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct ReadBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub block_query: BlockQuery,
    pub has_vector: bool,
    pub has_payload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct CreateBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct DeleteBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub block_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct UpdateBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct SearchBlocksInWorkspaceRequest {
    pub system_configurations: SystemConfigurations,
    pub search_method: SupportedSearchMethod,
    pub block_ids: Vec<Uuid>,
    pub query: Option<String>,
    pub query_vector: Option<Vec<f32>>,
    pub top_n: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct RequestReindexBlocksRequest {
    pub system_configurations: SystemConfigurations,
}

/// After done a `ReindexBlocksRequest`,
/// Use this to send the embedded blocks back to the server.
#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct SendReindexedBlocksRequest {
    pub system_configurations: SystemConfigurations,
    /// Rindexed / embedded blocks
    pub blocks: Vec<Block>,
}

/// It is recommended to send multiple assets in multiple attempts,
/// rather than in one attempt to avoid traffic overflows.
#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct SendAssetsToServerRequest {
    pub system_configurations: SystemConfigurations,

    /// The `storage_reference` looks like this: `<sha256>/<filename>.<extension>`.
    pub storage_reference: String,

    /// The asset bytes.
    pub bytes: Vec<u8>,
}

/// List what assets are available on the remote server.
#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct ListServerAssetsRequest {
    pub system_configurations: SystemConfigurations,
}

/// Buffer assets from the remote server's assets storage to the local.
#[derive(Debug, Clone, Serialize, Deserialize, GetSystemConfigurations)]
pub struct BufferAssetsFromServerRequest {
    pub system_configurations: SystemConfigurations,

    /// The `storage_reference` looks like this: `<sha256>/<filename>.<extension>`.
    pub storage_reference: String,
}
