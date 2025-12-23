use anyhow::Result;
use log::info;
use qdrant_client::{
    Qdrant,
    config::QdrantConfig,
    qdrant::{
        CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, FieldType,
        SparseVectorParamsBuilder, SparseVectorsConfigBuilder, TextIndexParamsBuilder,
        TokenizerType, VectorParamsBuilder, VectorsConfigBuilder,
    },
};

use crate::{
    configurations::system::Config,
    documents::{
        document_chunk::DocumentChunk,
        traits::{GetIndexableFields, IndexableField},
    },
};

#[derive(Clone)]
pub struct Database {
    client: Qdrant,
}

impl Database {
    pub async fn new(configuration: &Config) -> Result<Self> {
        let qdrant_config: QdrantConfig = QdrantConfig::from_url(&configuration.database.base_url);
        let client: Qdrant = Qdrant::new(qdrant_config)?;

        let mut dense_text_vector_config = VectorsConfigBuilder::default();
        dense_text_vector_config.add_named_vector_params(
            "dense_text_vector",
            VectorParamsBuilder::new(
                configuration.embedder.dimensions as u64,
                qdrant_client::qdrant::Distance::Cosine,
            ),
        );

        let mut sparse_vector_config = SparseVectorsConfigBuilder::default();
        sparse_vector_config
            .add_named_vector_params("sparse_text_vector", SparseVectorParamsBuilder::default());

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
                let error_string: String = error.to_string();

                // we don't want to panic out if the collection has already existed
                if error_string.contains("already exists") {
                    info!("Collection `note` has already existed. Skip creation");
                    return Ok(Self { client });
                }

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

        Ok(Self { client })
    }

    pub fn get_client(&self) -> &Qdrant {
        &self.client
    }
}
