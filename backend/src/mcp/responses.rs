use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;


#[derive(Debug, Serialize, JsonSchema)]
pub struct MCPServiceGenericResponse {
    #[schemars(description = "results")]
    pub results: Option<Value>,
}
