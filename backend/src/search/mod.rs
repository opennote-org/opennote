use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod keyword;
pub mod semantic;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SearchScopeIndicator {
    #[schemars(description = "in which range, you want to search")]
    pub search_scope: SearchScope,
    
    #[schemars(description = "respective id of the search scope. for AI, leave the id empty string when searching userspace")]
    pub id: String,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, JsonSchema, PartialEq, PartialOrd, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    Document,
    Collection,
    Userspace,
}
