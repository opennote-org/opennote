use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SupportedSearchMethod {
    Keyword,
    Semantic,
}

impl Display for SupportedSearchMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(&self).unwrap())
    }
}

impl Default for SupportedSearchMethod {
    fn default() -> Self {
        Self::Semantic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(default)]
pub struct UserSearchConfiguration {
    /// The default way of searching
    pub default_search_method: SupportedSearchMethod,

    /// Maximum size of chunks for search indexing. Adjust this if the value is beyond the model context limit
    pub document_chunk_size: usize,

    /// How many search results to get after typing in a search query
    pub top_n: usize,
}

impl Default for UserSearchConfiguration {
    fn default() -> Self {
        Self {
            document_chunk_size: 150,
            default_search_method: SupportedSearchMethod::Semantic,
            top_n: 10,
        }
    }
}
