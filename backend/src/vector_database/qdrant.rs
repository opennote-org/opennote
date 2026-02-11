use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::future::join_all;
use log::info;
use qdrant_client::{
    Qdrant,
    config::QdrantConfig,
    qdrant::{
        CollectionExistsRequest, Condition, CreateCollectionBuilder,
        CreateFieldIndexCollectionBuilder, DeleteCollectionBuilder, DeletePointsBuilder, FieldType,
        Filter, GetCollectionInfoRequest, GetPointsBuilder, PointId, PointStruct,
        ScrollPointsBuilder, SparseVectorParamsBuilder, SparseVectorsConfigBuilder,
        TextIndexParamsBuilder, TokenizerType, UpsertPointsBuilder, VectorParamsBuilder,
        VectorsConfigBuilder,
    },
};

use crate::{
    configurations::system::{Config, DatabaseConfig, EmbedderConfig},
    constants::{
        QDRANT_DENSE_TEXT_VECTOR_NAMED_PARAMS_NAME, QDRANT_SPARSE_TEXT_VECTOR_NAMED_PARAMS_NAME,
    },
    vector_database::traits::VectorDatabase,
    documents::{
        document_chunk::DocumentChunk,
        traits::{GetIndexableFields, IndexableField},
    },
    embedder::send_vectorization,
};

#[derive(Clone)]
pub struct QdrantDatabase {
    index: String,
    client: Qdrant,
}

#[async_trait]
impl VectorDatabase for QdrantDatabase {
    async fn add_document_chunks_to_database(
        &mut self,
        embedder_config: &EmbedderConfig,
        database_config: &DatabaseConfig,
        chunks: Vec<DocumentChunk>,
    ) -> Result<()> {
        // Vectorize the chunks
        // - Split the chunks into batches
        // - Vectorize batch by batch
        // - Batch is configurable
        let mut batches: Vec<Vec<DocumentChunk>> = Vec::new();
        let mut batch: Vec<DocumentChunk> = Vec::new();
        for chunk in chunks {
            if batch.len() == embedder_config.vectorization_batch_size {
                batches.push(batch);
                batch = Vec::new();
            }

            batch.push(chunk);
        }

        if !batch.is_empty() {
            batches.push(batch);
        }

        // Record the data entries
        let mut tasks = Vec::new();
        for batch in batches.into_iter() {
            tasks.push(send_vectorization(
                &embedder_config.provider,
                &embedder_config.base_url,
                &embedder_config.api_key,
                &embedder_config.model,
                &embedder_config.encoding_format,
                batch,
            ));
        }

        let results: Vec<std::result::Result<Vec<DocumentChunk>, anyhow::Error>> =
            join_all(tasks).await;
        let mut chunks: Vec<DocumentChunk> = Vec::new();
        for result in results {
            let result = result?;
            chunks.extend(result);
        }

        let points: Vec<PointStruct> = chunks
            .into_iter()
            .map(|chunk| PointStruct::from(chunk))
            .collect();

        self.client
            .upsert_points(UpsertPointsBuilder::new(&database_config.index, points).wait(true))
            .await?;

        Ok(())
    }

    async fn reindex_documents(&mut self, configuration: &Config) -> Result<()> {
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
        &mut self,
        database_config: &DatabaseConfig,
        document_ids: Vec<String>,
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
