use rmcp::{Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    documents::document_chunk::DocumentChunkSearchResult,
    search::semantic::search_documents_semantically,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MCPSearchRequest {
    #[schemars(description = "keywords, phrases or sentences you may want to search")]
    pub search_phrase: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MCPSearchResponse {
    #[schemars(description = "search results")]
    pub results: Vec<Value>,
}

pub struct MCPSearch;

#[tool_router]
impl MCPSearch {
    #[tool(description = "Semantically search the OpenNote documents")]
    pub fn semantic_search(
        &self,
        Parameters(MCPSearchRequest { search_phrase }): Parameters<MCPSearchRequest>,
    ) -> Json<MCPSearchResponse> {
        search_documents_semantically(
            client,
            document_metadata_ids,
            index,
            query,
            top_n,
            provider,
            base_url,
            api_key,
            model,
            encoding_format,
        );
        Ok(())
    }
}
