use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    database::traits::database::Database, metadata_storage::MetadataStorage,
    search::document_search_results::DocumentChunkSearchResult,
};

#[async_trait]
pub trait KeywordSearch {
    async fn search_documents(
        &self,
        database: &Arc<dyn Database>,
        document_metadata_ids: Vec<String>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<DocumentChunkSearchResult>>;
}
