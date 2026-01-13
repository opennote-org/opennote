//! This file defines configurations that are modifiable by individual users.
//! These are not necessarily break changes to the global uses,
//! but will directly affect the user him/herself.

use schemars::JsonSchema;
/// Each feature module will provide configurable options
/// The configurable options are collected into the configuration module
/// When corresponding requests are sent, the relevant configurations are collected
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SupportedSearchMethod {
    Keyword,
    Semantic,
}

impl Default for SupportedSearchMethod {
    fn default() -> Self {
        Self::Semantic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct UserConfigurations {
    pub search: UserSearchConfiguration,
}

impl Default for UserConfigurations {
    fn default() -> Self {
        Self {
            search: UserSearchConfiguration::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(default)]
pub struct UserSearchConfiguration {
    pub default_search_method: SupportedSearchMethod,
    pub document_chunk_size: usize,
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
