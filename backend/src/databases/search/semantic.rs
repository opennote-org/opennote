use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::{
    databases::database::traits::database::Database,
    search::document_search_results::DocumentChunkSearchResult,
};

#[async_trait]
pub trait SemanticSearch {
    async fn search_documents_semantically(
        &self,
        database: &Arc<dyn Database>,
        document_metadata_ids: Vec<String>,
        query: &str,
        top_n: usize,
        provider: &str,
        base_url: &str,
        api_key: &str,
        model: &str,
        encoding_format: &str,
    ) -> Result<Vec<DocumentChunkSearchResult>>;
}
