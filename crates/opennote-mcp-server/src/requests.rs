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
        description = "Get an overview of all blocks. Leave it empty to get all blocks. Specify block ids to see the specified blocks."
    )]
    pub block_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPReadBlockDetailsRequest {
    #[schemars(description = "Get the full content of a block")]
    pub block_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub enum MCPUpdateBlocksInsertionOperation {
    Start,
    End,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPUpdateBlocksInsertion {
    #[schemars(description = "append the text at the end of the block or insert at the start")]
    pub operation: MCPUpdateBlocksInsertionOperation,

    #[schemars(description = "the text that you want to insert or append")]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPUpdateBlocksReplacement {
    #[schemars(description = "the regex you use to select the text you want to replace")]
    pub regex: String,

    #[schemars(description = "the text that you want to replace with")]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPUpdateBlocksRequest {
    #[schemars(description = "the id of the block that you want to update for")]
    pub block_id: String,

    #[schemars(description = "insert or append text at the start or the end of the block. ")]
    pub insert: Option<MCPUpdateBlocksInsertion>,

    #[schemars(
        description = "if you want to replace certain text of the block, put the target here. This uses regex."
    )]
    pub replace: Option<MCPUpdateBlocksReplacement>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPCreateBlockRequest {
    #[schemars(description = "on which server, you will create the blocks? `local` for local")]
    pub scope: String,

    #[schemars(
        description = "which block would you like this to be its parent of? Leave None for no parent"
    )]
    pub parent_block_id: Option<String>,

    #[schemars(description = "the title of the block")]
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MCPDeleteBlocksRequest {
    #[schemars(description = "blocks that you would like to delete")]
    pub block_ids: Vec<String>,

    #[schemars(description = "on which server, you will create the blocks? `local` for local")]
    pub scope: String,
}
