use schemars::JsonSchema;
use serde::Deserialize;

use crate::search::SearchScopeIndicator;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MCPGetCollectionMetadata {
    #[schemars(description = "the collection's metadata id")]
    pub collection_metadata_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPSearchDocumentRequest {
    #[schemars(description = "keywords, phrases or sentences you may want to search")]
    pub query: String,
    
    #[schemars(description = "number of results you want. 20 is recommended for first try")]
    pub top_n: usize,
    
    #[schemars(description = "in which range, you want to search")]
    #[serde(flatten)]
    pub scope: SearchScopeIndicator,
}