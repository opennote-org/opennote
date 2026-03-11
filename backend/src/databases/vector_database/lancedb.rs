use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use arrow_array::{RecordBatch, RecordBatchIterator};
use async_trait::async_trait;
use lancedb::{
    arrow::{
        RecordBatchReader,
        arrow_schema::{FieldRef, Schema},
    },
    connect,
    index::{Index, scalar::FtsIndexBuilder, vector::IvfHnswSqIndexBuilder},
};
use serde_arrow::schema::{SchemaLike, TracingOptions};

use crate::{
    configurations::system::{Config, VectorDatabaseConfig},
    databases::database::{
        filters::{get_collections::GetCollectionFilter, get_documents::GetDocumentFilter},
        traits::database::Database,
    },
    databases::{
        search::{
            document_search_results::DocumentChunkSearchResult, keyword::KeywordSearch,
            semantic::SemanticSearch,
        },
        vector_database::traits::VectorDatabase,
    },
    documents::{
        collection_metadata::CollectionMetadata, document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    },
    embedder::send_vectorization,
};

pub struct LanceDB {
    vector_database: lancedb::Connection,
    schema: Arc<Schema>,
    fields: Vec<Arc<lancedb::arrow::arrow_schema::Field>>,
}

#[async_trait]
impl VectorDatabase for LanceDB {
    async fn validate_data_integrity(
        &self,
        vector_database_config: &VectorDatabaseConfig,
    ) -> Result<bool> {
        let table = self
            .vector_database
            .open_table(&vector_database_config.index)
            .execute()
            .await?;
        let rows = table.count_rows(None).await?;
        // Vector database should never have zero records when they are in normal use
        if rows == 0 {
            return Ok(false);
        }

        Ok(true)
    }

    async fn add_document_chunks_to_database(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        chunks: Vec<DocumentChunk>,
    ) -> Result<()> {
        let table = self
            .vector_database
            .open_table(&vector_database_config.index)
            .execute()
            .await?;
        
        let batch = self.convert_document_chunks_to_record_batch(&chunks)?;
        let iter = vec![batch].into_iter().map(Ok);
        let iterator = RecordBatchIterator::new(iter, self.schema.clone());

        table.add(iterator).execute().await?;

        Ok(())
    }

    async fn reindex_documents(&self, configuration: &Config) -> Result<()> {
        let table = self
            .vector_database
            .open_table(&vector_database_config.index)
            .execute()
            .await?;
        
        table.query().
        
        Ok(())
    }

    async fn delete_documents_from_database(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        document_ids: &Vec<String>,
    ) -> Result<()> {
        let mut vector_database = self.vector_database.lock().await;

        let chunk_ids: Vec<String> = vector_database
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

        vector_database.delete(&chunk_ids);

        vector_database.save()?;
        Ok(())
    }

    async fn get_document_chunks(
        &self,
        document_chunks_ids: Vec<String>,
    ) -> Result<Vec<DocumentChunk>> {
        let vector_database = self.vector_database.lock().await;

        // Acquire chunk ids
        let acquired_chunks: Vec<DocumentChunk> = vector_database
            .get(&document_chunks_ids)
            .into_iter()
            .map(|item| item.clone().into())
            .collect();

        Ok(acquired_chunks)
    }
}

#[async_trait]
impl SemanticSearch for LanceDB {
    async fn search_documents_semantically(
        &self,
        database: &Arc<dyn Database>,
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

        let vector_database = self.vector_database.lock().await;

        let results: Vec<HashMap<String, serde_json::Value>> = vector_database.query(
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
            &database
                .get_collections(&GetCollectionFilter::default(), false)
                .await?
                .into_iter()
                .map(|item| (item.id.clone(), item))
                .collect(),
            &database
                .get_documents(&GetDocumentFilter::default())
                .await?
                .into_iter()
                .map(|item| (item.id.clone(), item))
                .collect(),
        );

        Ok(results)
    }
}

#[async_trait]
impl KeywordSearch for LanceDB {}

impl LanceDB {
    pub async fn new(configuration: &Config) -> Result<Self> {
        let vector_database: lancedb::Connection = connect(&configuration.vector_database.base_url)
            .execute()
            .await?;

        let fields: Vec<Arc<lancedb::arrow::arrow_schema::Field>> =
            Vec::<FieldRef>::from_type::<DocumentChunk>(TracingOptions::default())?;
        let schema: Arc<Schema> = Arc::new(Schema::new(fields.clone()));

        match vector_database
            .create_empty_table(&configuration.vector_database.base_url, schema.clone())
            .mode(lancedb::database::CreateTableMode::Create)
            .execute()
            .await
        {
            Ok(_) => {}
            Err(_) => {
                log::info!("Table has created. Skip creation")
            }
        }

        let table = vector_database
            .open_table(&configuration.vector_database.base_url)
            .execute()
            .await?;

        table
            .create_index(
                &["dense_text_vector"],
                Index::IvfHnswSq(IvfHnswSqIndexBuilder::default()),
            )
            .execute()
            .await?;

        table
            .create_index(&["content"], Index::FTS(FtsIndexBuilder::default()))
            .execute()
            .await?;

        Ok(Self {
            vector_database,
            schema,
            fields,
        })
    }

    pub fn convert_document_chunks_to_record_batch(
        &self,
        chunks: &Vec<DocumentChunk>,
    ) -> Result<RecordBatch> {
        Ok(serde_arrow::to_record_batch(&self.fields, chunks)?)
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
