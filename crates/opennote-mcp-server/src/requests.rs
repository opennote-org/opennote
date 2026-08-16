use schemars::JsonSchema;
use serde::Deserialize;

use opennote_models::configurations::fields::search::SupportedSearchMethod;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPSearchRequest {
    #[schemars(description = "which search method you will use. semantic or keyword.")]
    pub search_method: SupportedSearchMethod,

    #[schemars(description = "search across a list of blocks. specify their ids here.")]
    pub block_ids: Vec<String>,

    #[schemars(description = "keywords, phrases or sentences you may want to search")]
    pub query: String,

    #[schemars(description = "number of results you want. 20 is recommended for first try")]
    pub top_n: usize,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPReadBlocksRequest {
    #[schemars(
        description = "search across a list of blocks. specify their ids here. Leave if empty to get all data. "
    )]
    pub block_ids: Vec<String>,
}
