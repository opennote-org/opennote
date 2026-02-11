use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::MutexGuard;

use crate::{
    documents::document_chunk::DocumentChunkSearchResult, metadata_storage::MetadataStorage,
};

#[async_trait]
pub trait SemanticSearch {
    async fn search_documents_semantically(
        &self,
        metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
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
