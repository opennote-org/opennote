use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{database::traits::database::Database, search::models::SearchResult};

#[async_trait]
pub trait KeywordSearch {
    async fn search_documents(
        &self,
        database: &Arc<dyn Database>,
        correspondent_ids: &Vec<Uuid>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<SearchResult>>;
}
