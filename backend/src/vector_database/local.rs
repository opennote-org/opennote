use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::future::join_all;
use local_vector_database::{Data, LocalVectorDatabase};
use log::info;
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    configurations::system::{Config, DatabaseConfig, EmbedderConfig},
    documents::{
        collection_metadata::CollectionMetadata,
        document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
        traits::{GetIndexableFields, IndexableField},
    },
    embedder::{send_vectorization, vectorize},
    metadata_storage::MetadataStorage,
    search::{document_search_results::DocumentChunkSearchResult, keyword::KeywordSearch, semantic::SemanticSearch},
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
        let qdrant_config: QdrantConfig = QdrantConfig::from_url(&configuration.database.base_url)
            // Timeout for preventing Qdrant killing time-consuming operations
            .timeout(std::time::Duration::from_secs(1000));
        let client: Qdrant = Qdrant::new(qdrant_config)?;

        if client
            .collection_exists(CollectionExistsRequest {
                collection_name: configuration.database.index.to_string(),
            })
            .await?
        {
            info!("Collection `note` has already existed. Skip creation");
            return Ok(Self {
                index: configuration.database.index.clone(),
                client,
            });
        }

        create_collection(&client, configuration).await?;

        match validate_configuration(&client, configuration).await {
            Ok(_) => {}
            Err(error) => {
                if error.to_string().contains("Mismatched") {
                    log::warn!("{}", error);
                } else {
                    log::info!("{}", error);
                    return Err(error);
                }
            }
        }

        Ok(Self {
            index: configuration.database.index.clone(),
            client,
        })
    }
}

async fn validate_configuration(qdrant_client: &Qdrant, configuration: &Config) -> Result<()> {
    match qdrant_client
        .collection_info(GetCollectionInfoRequest {
            collection_name: configuration.database.index.to_string(),
        })
        .await
    {
        Ok(result) => {
            let error_message: &'static str = "Collection configuration is missing, please check if the program has been configured properly";
            let info = result.result.expect(error_message);
            let config = info.config.expect(error_message);
            let params = config.params.expect(error_message);
            let vectors_config = params.vectors_config.expect(error_message);
            let vectors_config = vectors_config.config.expect(error_message);
            let size_in_collection = match vectors_config {
                qdrant_client::qdrant::vectors_config::Config::Params(params) => params.size,
                qdrant_client::qdrant::vectors_config::Config::ParamsMap(params) => {
                    if let Some(dense_vector_params) =
                        params.map.get(QDRANT_DENSE_TEXT_VECTOR_NAMED_PARAMS_NAME)
                    {
                        dense_vector_params.size
                    } else {
                        return Err(anyhow!("Misconfigured. Please raise an issue on GitHub"));
                    }
                }
            };

            if size_in_collection != configuration.embedder.dimensions as u64 {
                return Err(anyhow!(
                    "Collection uses {} dimensional vecotor, but config uses {}. Mismatched",
                    size_in_collection,
                    configuration.embedder.dimensions
                ));
            }
        }
        Err(error) => return Err(error.into()),
    }

    Ok(())
}

pub fn build_conditions(document_metadata_ids: Vec<String>) -> Vec<Condition> {
    document_metadata_ids
        .into_iter()
        .map(|id| Condition::matches("document_metadata_id", id.to_string()))
        .collect()
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
