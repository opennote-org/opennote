use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use opennote_embedder::entry::EmbedderEntry;

use crate::{database::traits::database::Database, search::models::SearchResult};

#[async_trait]
pub trait SemanticSearch {
    async fn search_documents_semantically(
        &self,
        database: &Arc<dyn Database>,
        correspondent_ids: &Vec<Uuid>,
        query: &str,
        top_n: usize,
        embedder_entry: &EmbedderEntry,
    ) -> Result<Vec<SearchResult>>;
}
