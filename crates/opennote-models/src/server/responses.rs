use actix_web::HttpResponse;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseResponse {
    pub status: bool,
    pub message: Option<String>,
    pub data: Option<Value>,
}

pub fn create_base_response<T>(results: Result<T>) -> HttpResponse
where
    T: Serialize,
{
    match results {
        Ok(results) => HttpResponse::Ok().json(BaseResponse {
            status: true,
            message: None,
            data: Some(serde_json::to_value(results).unwrap()),
        }),
        Err(error) => HttpResponse::Ok().json(BaseResponse {
            status: false,
            message: Some(error.to_string()),
            data: None,
        }),
    }
}
