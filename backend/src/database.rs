use anyhow::{Result, anyhow};
use log::info;
use qdrant_client::{
    Qdrant,
    config::QdrantConfig,
    qdrant::{
        CollectionExistsRequest, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
        DeleteCollectionBuilder, FieldType, GetCollectionInfoRequest, ScrollPointsBuilder,
        SparseVectorParamsBuilder, SparseVectorsConfigBuilder, TextIndexParamsBuilder,
        TokenizerType, VectorParamsBuilder, VectorsConfigBuilder,
    },
};

use crate::{
    configurations::system::Config,
    constants::{
        QDRANT_DENSE_TEXT_VECTOR_NAMED_PARAMS_NAME, QDRANT_SPARSE_TEXT_VECTOR_NAMED_PARAMS_NAME,
    },
    documents::{
        document_chunk::DocumentChunk,
        operations::add_document_chunks_to_database,
        traits::{GetIndexableFields, IndexableField},
    },
};

#[derive(Clone)]
pub struct Database {
    client: Qdrant,
}

impl Database {
    pub async fn validate_configuration(
        qdrant_client: &Qdrant,
        configuration: &Config,
    ) -> Result<()> {
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

    pub async fn create_collection(client: &Qdrant, configuration: &Config) -> Result<()> {
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
                    let text_index_params =
                        TextIndexParamsBuilder::new(TokenizerType::Multilingual)
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
            return Ok(Self { client });
        }

        Self::create_collection(&client, configuration).await?;

        match Self::validate_configuration(&client, configuration).await {
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

        Ok(Self { client })
    }

    pub fn get_client(&self) -> &Qdrant {
        &self.client
    }
}

pub async fn reindex_documents(client: &Qdrant, configuration: &Config) -> Result<()> {
    let counts: u64 = match client
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

    let retrieved_points = client
        .scroll(
            ScrollPointsBuilder::new(configuration.database.index.clone())
                .with_payload(true)
                .limit(counts as u32)
                .build(),
        )
        .await?
        .result;

    client
        .delete_collection(DeleteCollectionBuilder::new(
            configuration.database.index.clone(),
        ))
        .await?;

    Database::create_collection(client, configuration).await?;

    let document_chunks: Vec<DocumentChunk> = retrieved_points
        .into_iter()
        .map(|item| item.into())
        .collect();

    add_document_chunks_to_database(
        client,
        &configuration.embedder,
        &configuration.database,
        document_chunks,
    )
    .await?;

    Ok(())
}
