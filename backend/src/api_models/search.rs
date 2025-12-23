//! It defines request and response API models for search

use serde::{Deserialize, Serialize};

use crate::search::SearchScopeIndicator;

/// region: request

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchDocumentRequest {
    pub query: String,
    pub top_n: usize,
    pub scope: SearchScopeIndicator,
}
