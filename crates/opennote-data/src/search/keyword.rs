use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{database::traits::database::Database, search::models::RawSearchResult};

#[async_trait]
pub trait KeywordSearch {
    async fn search_documents(
        &self,
        database: &Arc<dyn Database>,
        payload_ids: &Vec<Uuid>,
        query: &str,
        _top_n: usize,
    ) -> Result<Vec<RawSearchResult>> {
        let chunks = database.search(query, payload_ids).await?;

        let mut results = Vec::new();
        for chunk in chunks.into_iter() {
            results.push(RawSearchResult {
                score: 0.0,
                block_id: chunk.block_id,
                payload_id: chunk.id,
            });
        }

        Ok(results)
    }
}
