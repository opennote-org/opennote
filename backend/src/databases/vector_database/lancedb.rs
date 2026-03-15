use std::sync::Arc;
use std::usize;
use std::{collections::HashMap, pin::Pin};

use anyhow::Result;
use arrow_array::{
    FixedSizeListArray, Float32Array, LargeStringArray, RecordBatch, RecordBatchIterator,
};
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
use serde_arrow::schema::{SchemaLike, TracingOptions};

use crate::{
    configurations::system::{Config, VectorDatabaseConfig},
    databases::{
        database::{
            filters::{get_collections::GetCollectionFilter, get_documents::GetDocumentFilter},
            traits::database::Database,
        },
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
    table_name: String,
    schema: Arc<Schema>,
    fields: Vec<Arc<lancedb::arrow::arrow_schema::Field>>,
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
                log::info!("Table has created. Skip creation")
            }
        }

        let table = self.vector_database.open_table(index).execute().await?;

        table
            .create_index(&["content"], Index::FTS(FtsIndexBuilder::default()))
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

    async fn add_document_chunks_to_database(
        &self,
        index: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<()> {
        let table = self.vector_database.open_table(index).execute().await?;

        let batch = self.convert_document_chunks_to_record_batch(&chunks)?;
        let iter = vec![batch].into_iter().map(Ok);
        let iterator = RecordBatchIterator::new(iter, self.schema.clone());

        table.add(iterator).execute().await?;

        Ok(())
    }

    async fn delete_index(&self, index: &str) -> Result<()> {
        self.vector_database.drop_table(index, &[]).await?;

        Ok(())
    }

    async fn delete_documents_from_database(
        &self,
        vector_database_config: &VectorDatabaseConfig,
        document_ids: &Vec<String>,
    ) -> Result<()> {
        let table = self
            .vector_database
            .open_table(&vector_database_config.index)
            .execute()
            .await?;

        let predicate: String = format!(
            "document_metadata_id IN ({})",
            document_ids
                .iter()
                .map(|id| format!("'{}'", id))
                .collect::<Vec<String>>()
                .join(", ")
        );

        table.delete(&predicate).await?;

        Ok(())
    }

    async fn get_document_chunks(
        &self,
        document_chunks_ids: Vec<String>,
    ) -> Result<Vec<DocumentChunk>> {
        let table = self
            .vector_database
            .open_table(&self.table_name)
            .execute()
            .await?;

        let predicate: String = format!(
            "document_metadata_id IN ({})",
            document_chunks_ids
                .iter()
                .map(|id| format!("'{}'", id))
                .collect::<Vec<String>>()
                .join(", ")
        );

        let stream = table.query().only_if(&predicate).execute().await?;

        let acquired_chunks = self.convert_record_batch_to_document_chunks(stream).await?;

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

        let table = self
            .vector_database
            .open_table(&self.table_name)
            .execute()
            .await?;

        let stream = table
            .vector_search(chunks[0].dense_text_vector.clone())?
            .distance_type(lancedb::DistanceType::Cosine)
            .limit(i64::MAX as usize) // LanceDB won't return exhaustive list like Qdrant
            .execute()
            .await?;

        let document_chunks: Vec<DocumentChunk> = self
            .convert_record_batch_to_document_chunks(stream)
            .await?
            .into_iter()
            .filter(|item| document_metadata_ids.contains(&item.document_metadata_id))
            .collect();

        let results: Vec<DocumentChunkSearchResult> = build_search_results(
            document_chunks[..top_n.min(document_chunks.len())].to_vec(),
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
impl KeywordSearch for LanceDB {
    async fn search_documents(
        &self,
        database: &Arc<dyn Database>,
        document_metadata_ids: &Vec<String>,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<DocumentChunkSearchResult>> {
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

        let document_chunks: Vec<DocumentChunk> = self
            .convert_record_batch_to_document_chunks(stream)
            .await?
            .into_iter()
            .filter(|item| document_metadata_ids.contains(&item.document_metadata_id))
            .collect();

        let results: Vec<DocumentChunkSearchResult> = build_search_results(
            document_chunks[..top_n.min(document_chunks.len())].to_vec(),
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

impl LanceDB {
    pub async fn new(configuration: &Config) -> Result<Self> {
        let vector_database: lancedb::Connection = connect(&configuration.vector_database.base_url)
            .execute()
            .await?;

        let options = TracingOptions::default().overwrite(
            "dense_text_vector",
            serde_arrow::marrow::datatypes::Field {
                name: "dense_text_vector".into(),
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
            Vec::<FieldRef>::from_type::<DocumentChunk>(options)?;
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

    pub fn convert_document_chunks_to_record_batch(
        &self,
        chunks: &Vec<DocumentChunk>,
    ) -> Result<RecordBatch> {
        Ok(serde_arrow::to_record_batch(&self.fields, chunks)?)
    }

    pub async fn convert_record_batch_to_document_chunks(
        &self,
        mut stream: Pin<
            Box<dyn RecordBatchStream<Item = Result<RecordBatch, lancedb::Error>> + Send>,
        >,
    ) -> Result<Vec<DocumentChunk>> {
        let mut acquired_chunks = Vec::new();
        while let Some(next) = stream.next().await {
            let next = next?;
            let chunks: Vec<DocumentChunk> = serde_arrow::from_record_batch(&next)?; 
            acquired_chunks.extend(chunks);
        }
        
        Ok(acquired_chunks)
    }
}

/// To fill in the document and collection title
pub fn build_search_results(
    document_chunks: Vec<DocumentChunk>,
    collection_metadatas_from_storage: &HashMap<String, CollectionMetadata>,
    document_metadatas_from_storage: &HashMap<String, DocumentMetadata>,
) -> Vec<DocumentChunkSearchResult> {
    let mut results = Vec::new();

    for (index, point) in document_chunks.into_iter().enumerate() {
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

        // Manually compute the score by using `x = 1 - (n * 0.1)`
        result.score = 1.0 - (index as f32 * 0.1);

        results.push(result);
    }

    results
}
