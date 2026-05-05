use std::sync::Arc;
use std::usize;
use std::{collections::HashMap, pin::Pin};

use anyhow::{Context, Result};
use arrow_array::{RecordBatch, RecordBatchIterator};
use async_trait::async_trait;
use futures::StreamExt;
use lancedb::arrow::RecordBatchStream;
use lancedb::index::scalar::{FtsIndexBuilder, FullTextSearchQuery};
use lancedb::{
    arrow::arrow_schema::{FieldRef, Schema},
    connect,
    index::Index,
    query::{ExecutableQuery, QueryBase},
};
use serde::{Deserialize, Serialize};
use serde_arrow::schema::{SchemaLike, TracingOptions};
use uuid::Uuid;

use crate::search::models::RawSearchResult;
use crate::{
    database::traits::database::Database,
    search::{keyword::KeywordSearch, models::SearchResult, semantic::SemanticSearch},
    vector_database::traits::VectorDatabase,
};
use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};
use opennote_models::content_type::ContentType;
use opennote_models::{
    configurations::system::{SystemConfigurations, VectorDatabaseConfig},
    payload::{Payload, create_query},
};

pub struct LanceDB {
    vector_database: lancedb::Connection,
    table_name: String,
    schema: Arc<Schema>,
    fields: Vec<Arc<lancedb::arrow::arrow_schema::Field>>,
}

/// LancedDB sucks at handling non-string types in their query,
/// therefore, we need to create a struct for handling this
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadLanceDB {
    /// A unique identification of its owner block
    pub block_id: String,
    /// A unique identification of this payload
    pub id: String,
    /// When this payload is created
    pub created_at: i64,
    /// Last time this payload is modified
    pub last_modified: i64,
    /// Content type presented in which style. For example, text can be P1 or so.
    pub content_type: ContentType,
    /// Texts stored in payload. When saving jsons, it is recommended to also save a string json for indexing for searching
    pub texts: String,
    /// Bytes stored in payload. Typically, we modalities other than texts, like images and jsons
    pub bytes: Vec<u8>,
    /// Vector representation of the stored texts or bytes
    pub vector: Vec<f32>,
}

impl From<Payload> for PayloadLanceDB {
    fn from(value: Payload) -> Self {
        Self {
            block_id: value.block_id.to_string(),
            id: value.id.to_string(),
            created_at: value.created_at,
            last_modified: value.last_modified,
            content_type: value.content_type,
            texts: value.texts,
            bytes: value.bytes,
            vector: value.vector,
        }
    }
}

impl From<PayloadLanceDB> for Payload {
    fn from(value: PayloadLanceDB) -> Self {
        Self {
            block_id: Uuid::parse_str(&value.block_id).unwrap(),
            id: Uuid::parse_str(&value.id).unwrap(),
            created_at: value.created_at,
            last_modified: value.last_modified,
            content_type: value.content_type,
            texts: value.texts,
            bytes: value.bytes,
            vector: value.vector,
        }
    }
}

#[async_trait]
impl VectorDatabase for LanceDB {
    async fn create_index(&self, index: &str, _dimensions: usize) -> Result<()> {
        match self
            .vector_database
            .create_empty_table(index, self.schema.clone())
            .mode(lancedb::database::CreateTableMode::Create)
            .execute()
            .await
        {
            Ok(_) => {}
            Err(_) => {
                // Skip creation if the vector database has already been there
            }
        }

        let table = self.vector_database.open_table(index).execute().await?;

        table
            .create_index(
                &["texts"],
                Index::FTS(
                    FtsIndexBuilder::default()
                        .base_tokenizer("ngram".to_string())
                        .ngram_min_length(2),
                ),
            )
            .execute()
            .await?;

        Ok(())
    }

    async fn validate_data_integrity(
        &self,
        vector_database_config: &VectorDatabaseConfig,
    ) -> Result<bool> {
        let table = self
            .vector_database
            .open_table(&vector_database_config.index)
            .execute()
            .await?;
        let rows: usize = table.count_rows(None).await?;
        // Vector database should never have zero records when they are in normal use
        if rows == 0 {
            return Ok(false);
        }

        Ok(true)
    }

    async fn create_entries(&self, index: &str, payloads: Vec<Payload>) -> Result<()> {
        let batch = self.convert_payloads_to_record_batch(&payloads)?;
        let iter = vec![batch].into_iter().map(Ok);
        let iterator = RecordBatchIterator::new(iter, self.schema.clone());

        let table = self.vector_database.open_table(index).execute().await?;
        table.add(iterator).execute().await?;

        Ok(())
    }

    async fn delete_index(&self, index: &str) -> Result<()> {
        self.vector_database.drop_table(index, &[]).await?;

        Ok(())
    }

    async fn delete_entries(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        payload_ids: &Vec<Uuid>,
    ) -> Result<()> {
        let predicate: String = format!(
            "id IN ({})",
            payload_ids
                .iter()
                .map(|id| format!("'{}'", id))
                .collect::<Vec<String>>()
                .join(", ")
        );

        let table = self
            .vector_database
            .open_table(&vector_database_config.index)
            .execute()
            .await?;

        table.delete(&predicate).await?;

        Ok(())
    }
}

#[async_trait]
impl SemanticSearch for LanceDB {
    async fn search_documents_semantically(
        &self,
        payload_ids: &Vec<Uuid>, // payload ids
        query: &str,
        _top_n: usize,
        embedder_entry: &EmbedderEntry,
    ) -> Result<Vec<RawSearchResult>> {
        // Convert to vec
        let chunks = send_vectorization(vec![create_query(query)], embedder_entry).await?;

        let table = self
            .vector_database
            .open_table(&self.table_name)
            .execute()
            .await?;

        let stream = table
            .vector_search(chunks[0].vector.clone())?
            .distance_type(lancedb::DistanceType::Cosine)
            .limit(i64::MAX as usize) // LanceDB won't return exhaustive list like Qdrant
            .execute()
            .await?;

        let payloads: Vec<Payload> = self
            .convert_record_batch_to_payloads(stream)
            .await?
            .into_iter()
            .filter(|item| payload_ids.contains(&item.id))
            .collect();

        Ok(build_raw_search_results(payloads))
    }
}

#[async_trait]
impl KeywordSearch for LanceDB {
    async fn search_documents(
        &self,
        _database: &Arc<dyn Database>,
        payload_ids: &Vec<Uuid>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<RawSearchResult>> {
        let table = self
            .vector_database
            .open_table(&self.table_name)
            .execute()
            .await?;

        let stream = table
            .query()
            .full_text_search(FullTextSearchQuery::new(query.to_string()).limit(Some(top_n as i64)))
            .limit(i64::MAX as usize) // LanceDB won't return exhaustive lists like Qdrant
            .execute()
            .await?;

        let payloads: Vec<Payload> = self
            .convert_record_batch_to_payloads(stream)
            .await?
            .into_iter()
            .filter(|item| payload_ids.contains(&item.id))
            .collect();

        Ok(build_raw_search_results(payloads))
    }
}

impl LanceDB {
    pub async fn new(configuration: &SystemConfigurations) -> Result<Self> {
        let vector_database: lancedb::Connection = connect(&configuration.vector_database.base_url)
            .execute()
            .await?;

        let options = TracingOptions::default()
            .enums_without_data_as_strings(true)
            .overwrite(
                "vector",
                serde_arrow::marrow::datatypes::Field {
                    name: "vector".into(),
                    data_type: serde_arrow::marrow::datatypes::DataType::FixedSizeList(
                        Box::new(serde_arrow::marrow::datatypes::Field {
                            name: "item".into(),
                            data_type: serde_arrow::marrow::datatypes::DataType::Float32,
                            nullable: false,
                            metadata: HashMap::new(),
                        }),
                        configuration.embedder.dimensions as i32,
                    ),
                    nullable: false,
                    metadata: HashMap::new(),
                },
            )?;
        let fields: Vec<Arc<lancedb::arrow::arrow_schema::Field>> =
            Vec::<FieldRef>::from_type::<PayloadLanceDB>(options)?;
        let schema: Arc<Schema> = Arc::new(Schema::new(fields.clone()));

        let vector_database = Self {
            vector_database,
            table_name: configuration.vector_database.index.clone(),
            schema,
            fields,
        };

        vector_database
            .create_index(
                &configuration.vector_database.index,
                configuration.embedder.dimensions,
            )
            .await?;

        Ok(vector_database)
    }

    pub fn convert_payloads_to_record_batch(&self, chunks: &Vec<Payload>) -> Result<RecordBatch> {
        let chunks: Vec<PayloadLanceDB> = chunks
            .iter()
            .map(|item| PayloadLanceDB::from(item.clone()))
            .collect();
        Ok(serde_arrow::to_record_batch(&self.fields, &chunks)?)
    }

    pub async fn convert_record_batch_to_payloads(
        &self,
        mut stream: Pin<
            Box<dyn RecordBatchStream<Item = Result<RecordBatch, lancedb::Error>> + Send>,
        >,
    ) -> Result<Vec<Payload>> {
        let mut acquired_chunks = Vec::new();
        while let Some(next) = stream.next().await {
            let next = next?;
            let chunks: Vec<PayloadLanceDB> = serde_arrow::from_record_batch(&next)?;
            acquired_chunks.extend(chunks);
        }

        Ok(acquired_chunks
            .into_iter()
            .map(|item| item.into())
            .collect())
    }
}

fn build_raw_search_results(payloads: Vec<Payload>) -> Vec<RawSearchResult> {
    let total_number_results = payloads.len() as f32;

    payloads
        .into_iter()
        .enumerate()
        .map(|(index, item)| RawSearchResult {
            block_id: item.block_id,
            payload_id: item.id,
            score: 1.0 - (index as f32 / total_number_results),
        })
        .collect()
}

async fn build_search_results(
    database: &Arc<dyn Database>,
    payloads: Vec<Payload>,
) -> Result<Vec<SearchResult>> {
    if payloads.is_empty() {
        return Ok(Vec::new());
    }

    let total_number_results = payloads.len() as f32;
    let mut results = Vec::with_capacity(payloads.len());
    let mut paths_map = HashMap::new();

    // Only fetch the payloads with different block ids
    for (index, payload) in payloads.into_iter().enumerate() {
        // Check if the block id's path had already fetched
        if !paths_map.contains_key(&payload.block_id) {
            let path = database
                .read_block_path(payload.block_id)
                .await
                .context(format!("Failed reading blocks for {}", payload.block_id))?;
            // Use hashmap to store `block_id : path`
            paths_map.insert(payload.block_id, path);
        }

        // Manually compute the score by using `x = 1 - (n / m)`
        let result = SearchResult::new(
            paths_map
                .get(&payload.block_id)
                .expect("block_id was just inserted")
                .clone(),
            payload,
            1.0 - (index as f32 / total_number_results),
        );
        results.push(result);
    }

    Ok(results)
}
