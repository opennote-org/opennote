use qdrant_client::qdrant::Condition;
use serde::{Deserialize, Serialize};

pub mod keyword;
pub mod semantic;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchScopeIndicator {
    pub search_scope: SearchScope,
    pub id: String,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    Document,
    Collection,
    Userspace,
}

pub fn build_search_conditions(document_metadata_ids: Vec<String>) -> Vec<Condition> {
    document_metadata_ids
        .into_iter()
        .map(|id| Condition::matches("document_metadata_id", id.to_string()))
        .collect()
}
