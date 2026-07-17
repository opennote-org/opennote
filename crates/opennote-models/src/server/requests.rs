use actix_web::web::Bytes;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_encrypt::{
    EncryptedMessage, serialize::impls::BincodeSerializer, shared_key::SharedKey,
    traits::SerdeEncryptSharedKey,
};
use uuid::Uuid;

use crate::{block::Block, configurations::search::SupportedSearchMethod, query::BlockQuery};

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
    pub block_query: BlockQuery,
    pub has_vector: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlocksInWorkspaceRequest {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlocksInWorkspaceRequest {
    pub block_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBlocksInWorkspaceRequest {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBlocksInWorkspaceRequest {
    pub search_method: SupportedSearchMethod,
    pub block_ids: Vec<Uuid>,
    pub query: Option<String>,
    pub query_vector: Option<Vec<f32>>,
    pub top_n: usize,
}
