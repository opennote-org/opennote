use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::search::models::RawSearchResult;

#[async_trait]
pub trait SemanticSearch {
    async fn search_documents_semantically(
        &self,
        payload_ids: &Vec<Uuid>,
        query: &[f32],
        top_n: usize,
    ) -> Result<Vec<RawSearchResult>>;
}
