use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use local_vector_database::{Data, LocalVectorDatabase};
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    configurations::system::{Config, DatabaseConfig, EmbedderConfig},
    documents::{
        collection_metadata::CollectionMetadata,
        document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    },
    embedder::{send_vectorization, vectorize},
    metadata_storage::MetadataStorage,
    search::{
        document_search_results::DocumentChunkSearchResult, keyword::KeywordSearch,
        semantic::SemanticSearch,
    },
    vector_database::traits::VectorDatabase,
};

pub struct Local {
    database: Mutex<LocalVectorDatabase>,
}

#[async_trait]
impl VectorDatabase for Local {
    async fn add_document_chunks_to_database(
        &self,
        embedder_config: &EmbedderConfig,
        database_config: &DatabaseConfig,
        chunks: Vec<DocumentChunk>,
    ) -> Result<()> {
        let chunks: Vec<DocumentChunk> = vectorize(embedder_config, chunks).await?;

        let mut database = self.database.lock().await;

        database.upsert(chunks.into_iter().map(|item| item.into()).collect());

        Ok(())
    }

    async fn reindex_documents(&self, configuration: &Config) -> Result<()> {
        let database = self.database.lock().await;

        let retrieved_points = database.get_all_owned();
        let document_chunks: Vec<DocumentChunk> = retrieved_points
            .into_iter()
            .map(|item| item.into())
            .collect();

        self.add_document_chunks_to_database(
            &configuration.embedder,
            &configuration.database,
            document_chunks,
        )
        .await?;

        Ok(())
    }

    async fn delete_documents_from_database(
        &self,
        database_config: &DatabaseConfig,
        document_ids: &Vec<String>,
    ) -> Result<()> {
        let mut database = self.database.lock().await;

        let chunk_ids: Vec<String> = database
            .get_all()
            .iter()
            .filter(|item| {
                document_ids.contains(
                    &item
                        .fields
                        .get("document_metadata_id")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                )
            })
            .map(|item| item.id.clone())
            .collect();

        database.delete(&chunk_ids);

        Ok(())
    }

    async fn get_document_chunks(
        &self,
        document_chunks_ids: Vec<String>,
    ) -> Result<Vec<DocumentChunk>> {
        let database = self.database.lock().await;

        // Acquire chunk ids
        let acquired_chunks: Vec<DocumentChunk> = database
            .get(&document_chunks_ids)
            .into_iter()
            .map(|item| item.clone().into())
            .collect();

        Ok(acquired_chunks)
    }
}

#[async_trait]
impl SemanticSearch for Local {
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
    ) -> Result<Vec<DocumentChunkSearchResult>> {
        // Convert to vec
        let chunks: Vec<DocumentChunk> = send_vectorization(
            provider,
            base_url,
            api_key,
            model,
            encoding_format,
            vec![DocumentChunk::new(query.to_owned(), "", "")],
        )
        .await?;

        let database = self.database.lock().await;

        let results: Vec<HashMap<String, serde_json::Value>> = database.query(
            &chunks[0].dense_text_vector,
            top_n,
            None,
            Some(Box::new(move |item: &Data| {
                document_metadata_ids.contains(
                    &item
                        .fields
                        .get("document_metadata_id")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                )
            })),
        );

        let results: Vec<DocumentChunkSearchResult> = build_search_results(
            results,
            &metadata_storage.collections,
            &metadata_storage.documents,
        );

        Ok(results)
    }
}

#[async_trait]
impl KeywordSearch for Local {
    async fn search_documents(
        &self,
        metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
        document_metadata_ids: Vec<String>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<DocumentChunkSearchResult>> {
        let conditions: Vec<Condition> = build_conditions(document_metadata_ids);

        let response: ScrollResponse = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.index)
                    .filter(Filter {
                        should: conditions,
                        must: vec![Condition::matches_text_any("content", query)],
                        ..Default::default()
                    })
                    .limit(top_n as u32)
                    .build(),
            )
            .await?;

        let results: Vec<DocumentChunkSearchResult> = build_search_results(
            None,
            Some(response.result),
            &metadata_storage.collections,
            &metadata_storage.documents,
        );

        Ok(results)
    }
}

impl Local {
    pub async fn new(configuration: &Config) -> Result<Self> {
        Ok(Self {
            database: Mutex::new(
                LocalVectorDatabase::new(
                    configuration.embedder.dimensions,
                    &configuration.database.base_url,
                )
                .unwrap(),
            ),
        })
    }
}

/// To fill in the document and collection title
pub fn build_search_results(
    query_results: Vec<HashMap<String, serde_json::Value>>,
    collection_metadatas_from_storage: &HashMap<String, CollectionMetadata>,
    document_metadatas_from_storage: &HashMap<String, DocumentMetadata>,
) -> Vec<DocumentChunkSearchResult> {
    let mut results = Vec::new();

    for point in query_results {
        let mut result: DocumentChunkSearchResult = DocumentChunkSearchResult::from(point);

        if let Some(document_metadata) =
            document_metadatas_from_storage.get(&result.document_chunk.document_metadata_id)
        {
            result.document_title = Some(document_metadata.title.clone());
        }

        if let Some(collection_metadata) =
            collection_metadatas_from_storage.get(&result.document_chunk.collection_metadata_id)
        {
            result.collection_title = Some(collection_metadata.title.clone());
        }

        results.push(result);
    }

    results
}
