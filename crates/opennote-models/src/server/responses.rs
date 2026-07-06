use actix_web::HttpResponse;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_encrypt::{
    EncryptedMessage, serialize::impls::BincodeSerializer, shared_key::SharedKey,
    traits::SerdeEncryptSharedKey,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseResponse {
    pub status: bool,
    pub message: Option<String>,
    pub data: Option<Vec<u8>>,
}

pub fn create_bad_response(message: String) -> HttpResponse {
    HttpResponse::BadRequest().body(message)
}

pub fn create_base_response<T>(results: Result<T>, shared_key: &SharedKey) -> HttpResponse
where
    T: Serialize,
{
    let content: BaseResponse = match results {
        Ok(results) => BaseResponse {
            status: true,
            message: None,
            data: Some(serde_json::to_vec(&results).unwrap()),
        },
        Err(error) => BaseResponse {
            status: false,
            message: Some(error.to_string()),
            data: None,
        },
    };

    HttpResponse::Ok().body(content.encrypt(shared_key).unwrap().serialize())
}

pub async fn parse_base_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    shared_key: &SharedKey,
) -> Result<T> {
    let bytes: actix_web::web::Bytes = response
        .bytes()
        .await
        .context("Failed to deserialize BaseResponse")?;

    let encrypted_message: EncryptedMessage = EncryptedMessage::deserialize(bytes.to_vec())?;
    let base: BaseResponse = BaseResponse::decrypt_owned(&encrypted_message, &shared_key)?;

    if base.status {
        serde_json::from_slice(&base.data.unwrap_or_default())
            .context("Failed to deserialize response data")
    } else {
        bail!(
            base.message
                .unwrap_or_else(|| "Unknown server error".to_string())
        )
    }
}

impl SerdeEncryptSharedKey for BaseResponse {
    type S = BincodeSerializer<Self>;
}
