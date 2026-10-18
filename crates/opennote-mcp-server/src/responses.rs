use anyhow::{Error, Result};
use rmcp::Json;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize, JsonSchema)]
pub struct MCPServiceGenericResponse {
    #[schemars(description = "results")]
    pub results: Option<Value>,
}

pub fn create_mcp_server_response<T: Serialize>(
    result: Result<T, Error>,
) -> Json<MCPServiceGenericResponse> {
    let serialized = match result {
        Ok(result) => serde_json::to_value(result).unwrap(),
        Err(error) => {
            tracing::warn!("MCP service reported error: {}", error);
            serde_json::to_value(error.to_string()).unwrap()
        }
    };

    Json(MCPServiceGenericResponse {
        results: Some(serialized),
    })
}
