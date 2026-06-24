pub mod keyword;
pub mod models;
pub mod semantic;

use std::{fmt::Display, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SearchScopeIndicator {
    #[schemars(description = "in which range, you want to search")]
    pub search_scope: SearchScope,

    #[schemars(
        description = "respective id of the search scope. for AI, leave the id empty string when searching userspace"
    )]
    pub id: String,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, JsonSchema, PartialEq, PartialOrd, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    Document,
    Collection,
    Userspace,
}

impl FromStr for SearchScope {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Display for SearchScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(&self).unwrap())
    }
}
