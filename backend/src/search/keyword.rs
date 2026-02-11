use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::MutexGuard;

use crate::{
    documents::document_chunk::DocumentChunkSearchResult, metadata_storage::MetadataStorage,
};

#[async_trait]
pub trait KeywordSearch {
    async fn search_documents(
        &self,
        metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
        document_metadata_ids: Vec<String>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<DocumentChunkSearchResult>>;
}
