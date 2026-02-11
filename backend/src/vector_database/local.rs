use std::collections::HashMap;

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
        document_chunk::{DocumentChunk, DocumentChunkSearchResult},
        document_metadata::DocumentMetadata,
        traits::{GetIndexableFields, IndexableField},
    },
    embedder::{send_vectorization, vectorize},
    metadata_storage::MetadataStorage,
    search::{keyword::KeywordSearch, semantic::SemanticSearch},
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
        
        let database = self.database.lock().await;
        
        database.upsert(datas);
        
        let points: Vec<PointStruct> = chunks
            .into_iter()
            .map(|chunk| PointStruct::from(chunk))
            .collect();

        self.client
            .upsert_points(UpsertPointsBuilder::new(&database_config.index, points).wait(true))
            .await?;

        Ok(())
    }

    async fn reindex_documents(&self, configuration: &Config) -> Result<()> {
        let counts: u64 = match self
            .client
            .collection_info(GetCollectionInfoRequest {
                collection_name: configuration.database.index.to_string(),
            })
            .await?
            .result
        {
            Some(result) => result.points_count(),
            None => return Err(anyhow!("Cannot get points count. Re-indexation failed")),
        };

        // For now, this is only a simple implementation.
        // TODO: Should consider dealing with larger collection.
        if counts > u32::MAX as u64 {
            return Err(anyhow!(
                "Number of document chunks had exceeded the re-indexation limit {}",
                u32::MAX
            ));
        }

        let retrieved_points = self
            .client
            .scroll(
                ScrollPointsBuilder::new(configuration.database.index.clone())
                    .with_payload(true)
                    .limit(counts as u32)
                    .build(),
            )
            .await?
            .result;

        self.client
            .delete_collection(DeleteCollectionBuilder::new(
                configuration.database.index.clone(),
            ))
            .await?;

        create_collection(&self.client, configuration).await?;

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
        let mut conditions: Vec<Condition> = Vec::new();
        for id in document_ids.iter() {
            conditions.push(Condition::matches("document_metadata_id", id.to_owned()));
        }

        match self
            .client
            .delete_points(
                DeletePointsBuilder::new(&database_config.index)
                    .points(Filter::any(conditions))
                    .wait(true),
            )
            .await
        {
            Ok(_) => {}
            Err(error) => log::error!(
                "Qdrant cannot delete documents {:?} due to {}",
                document_ids,
                error
            ),
        }

        Ok(())
    }

    async fn get_document_chunks(
        &self,
        document_chunks_ids: Vec<String>,
    ) -> Result<Vec<DocumentChunk>> {
        // Acquire chunk ids
        let acquired_chunks: Vec<DocumentChunk> = match self
            .client
            .get_points(
                GetPointsBuilder::new(
                    &self.index,
                    document_chunks_ids
                        .into_iter()
                        .map(|chunk| chunk.into())
                        .collect::<Vec<PointId>>(),
                )
                .with_payload(true),
            )
            .await
        {
            Ok(result) => result
                .result
                .into_iter()
                .map(|point| point.into())
                .collect(),
            Err(error) => {
                return Err(error.into());
            }
        };

        Ok(acquired_chunks)
    }
}

#[async_trait]
impl SemanticSearch for QdrantDatabase {
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

        let conditions: Vec<Condition> = build_conditions(document_metadata_ids);

        let response = self
            .client
            .query(
                QueryPointsBuilder::new(&self.index)
                    .using("dense_text_vector")
                    .with_payload(true)
                    .query(chunks[0].dense_text_vector.to_owned())
                    .limit(top_n as u64)
                    .filter(Filter::any(conditions))
                    .params(SearchParamsBuilder::default().hnsw_ef(128).exact(false)),
            )
            .await?;

        let results: Vec<DocumentChunkSearchResult> = build_search_results(
            Some(response.result),
            None,
            &metadata_storage.collections,
            &metadata_storage.documents,
        );

        Ok(results)
    }
}

#[async_trait]
impl KeywordSearch for QdrantDatabase {
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

impl QdrantDatabase {
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

async fn create_collection(client: &Qdrant, configuration: &Config) -> Result<()> {
    let mut dense_text_vector_config = VectorsConfigBuilder::default();
    dense_text_vector_config.add_named_vector_params(
        QDRANT_DENSE_TEXT_VECTOR_NAMED_PARAMS_NAME,
        VectorParamsBuilder::new(
            configuration.embedder.dimensions as u64,
            qdrant_client::qdrant::Distance::Cosine,
        ),
    );

    let mut sparse_vector_config = SparseVectorsConfigBuilder::default();
    sparse_vector_config.add_named_vector_params(
        QDRANT_SPARSE_TEXT_VECTOR_NAMED_PARAMS_NAME,
        SparseVectorParamsBuilder::default(),
    );

    match client
        .create_collection(
            CreateCollectionBuilder::new(&configuration.database.index)
                .vectors_config(dense_text_vector_config)
                .sparse_vectors_config(sparse_vector_config)
                .build(),
        )
        .await
    {
        Ok(_) => info!("Created a new collection `note` to record document chunks"),
        Err(error) => {
            // we can't use the notebook without having a collection
            panic!("Failed to initialize collection due to: {}", error);
        }
    }

    // Create index for these fields that are potentially be filters.
    // This is to optimize the search performance when the user stores large datasets.
    for field in DocumentChunk::get_indexable_fields() {
        match field {
            IndexableField::Keyword(field) => {
                client
                    .create_field_index(CreateFieldIndexCollectionBuilder::new(
                        &configuration.database.index,
                        field,
                        FieldType::Keyword,
                    ))
                    .await?;
            }
            IndexableField::FullText(field) => {
                let text_index_params = TextIndexParamsBuilder::new(TokenizerType::Multilingual)
                    .phrase_matching(true)
                    .build();

                client
                    .create_field_index(
                        CreateFieldIndexCollectionBuilder::new(
                            &configuration.database.index,
                            field,
                            FieldType::Text,
                        )
                        .field_index_params(text_index_params),
                    )
                    .await?;
            }
        }
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
    scored_points: Option<Vec<ScoredPoint>>,
    retrieved_points: Option<Vec<RetrievedPoint>>,
    collection_metadatas_from_storage: &HashMap<String, CollectionMetadata>,
    document_metadatas_from_storage: &HashMap<String, DocumentMetadata>,
) -> Vec<DocumentChunkSearchResult> {
    let mut results = Vec::new();

    if let Some(points) = scored_points {
        for point in points {
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
    }

    if let Some(points) = retrieved_points {
        for point in points {
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
    }

    results
}
