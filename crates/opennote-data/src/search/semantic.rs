use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use opennote_embedder::entry::EmbedderEntry;

use crate::search::models::RawSearchResult;

#[async_trait]
pub trait SemanticSearch {
    async fn search_documents_semantically(
        &self,
        payload_ids: &Vec<Uuid>,
        query: &str,
        top_n: usize,
        embedder_entry: &EmbedderEntry,
    ) -> Result<Vec<RawSearchResult>>;
}
