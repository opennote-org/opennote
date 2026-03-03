use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    database::{
        filters::{get_collections::GetCollectionFilter, get_documents::GetDocumentFilter},
        traits::database::Database,
    },
    search::document_search_results::DocumentChunkSearchResult,
};

#[async_trait]
pub trait KeywordSearch {
    async fn search_documents(
        &self,
        database: &Arc<dyn Database>,
        document_metadata_ids: &Vec<String>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<DocumentChunkSearchResult>> {
        let chunks = database.search(query, document_metadata_ids).await?;

        let mut document_metadata_ids = Vec::new();
        let mut collection_metadata_ids = Vec::new();

        for chunk in chunks.iter() {
            document_metadata_ids.push(chunk.document_metadata_id.clone());
            collection_metadata_ids.push(chunk.collection_metadata_id.clone());
        }

        let collection_titles: Vec<String> = database
            .get_collections(
                &GetCollectionFilter {
                    ids: collection_metadata_ids,
                    ..Default::default()
                },
                false,
            )
            .await?
            .into_iter()
            .map(|item| item.title)
            .collect();

        let document_titles: Vec<String> = database
            .get_documents(&GetDocumentFilter {
                ids: document_metadata_ids,
                ..Default::default()
            })
            .await?
            .into_iter()
            .map(|item| item.title)
            .collect();

        let mut results = Vec::new();
        for (index, chunk) in chunks.into_iter().enumerate() {
            results.push(DocumentChunkSearchResult {
                score: 0.0,
                document_chunk: chunk,
                document_title: Some(document_titles[index].clone()),
                collection_title: Some(collection_titles[index].clone()),
            });
        }

        Ok(results)
    }
}
