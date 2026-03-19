use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::{
    databases::{
        database::traits::database::Database,
        search::document_search_results::DocumentChunkSearchResult,
    },
    embedders::entry::EmbedderEntry,
};

#[async_trait]
pub trait SemanticSearch {
    async fn search_documents_semantically(
        &self,
        database: &Arc<dyn Database>,
        document_metadata_ids: Vec<String>,
        query: &str,
        top_n: usize,
        embedder_entry: &EmbedderEntry,
    ) -> Result<Vec<DocumentChunkSearchResult>>;
}
