use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use arrow_array::RecordBatch;
use async_trait::async_trait;
use lancedb::{
    arrow::arrow_schema::{DataType, Field, Schema},
    connect,
    index::{Index, scalar::FtsIndexBuilder, vector::IvfHnswSqIndexBuilder},
};

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

        table.add(chunks).execute().await?;

        Ok(())
    }

    async fn reindex_documents(&self, configuration: &Config) -> Result<()> {
        let vector_database = self.vector_database.lock().await;

        let retrieved_points = vector_database.get_all_owned();
        let document_chunks: Vec<DocumentChunk> = retrieved_points
            .into_iter()
            .map(|item| item.into())
            .collect();

        self.add_document_chunks_to_database(&configuration.vector_database, document_chunks)
            .await?;

        vector_database.save()?;
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

        let schema: Arc<Schema> = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("document_metadata_id", DataType::Utf8, false),
            Field::new("collection_metadata_id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "dense_text_vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    configuration.embedder.dimensions as i32,
                ),
                false,
            ),
        ]));

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
        })
    }

    pub fn convert_document_chunks_to_record_batches(
        &self,
        chunks: Vec<DocumentChunk>,
    ) -> Vec<RecordBatch> {
        chunks
            .into_iter()
            .map(|item| RecordBatch::try_new(self.schema.clone(), vec![
                Arc::new(arrow_array::StringArray::from(item.id)),
                // Field::new("document_metadata_id", DataType::Utf8, false),
                // Field::new("collection_metadata_id", DataType::Utf8, false),
                // Field::new("content", DataType::Utf8, false),
                // Field::new(
                //     "dense_text_vector",
                //     DataType::FixedSizeList(
                //         Arc::new(Field::new("item", DataType::Float32, true)),
                //         configuration.embedder.dimensions as i32,
                //     ),
                //     false,
                // ),
            ]))
            .collect()
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
