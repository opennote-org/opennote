use std::sync::Arc;

use anyhow::Result;

use crate::{
    database::{filters::get_collections::GetCollectionFilter, traits::database::Database},
    documents::{
        collection_metadata::CollectionMetadata, document_chunk::DocumentChunk,
        document_metadata::DocumentMetadata,
    },
    search::SearchScope,
};

pub async fn retrieve_document_ids_by_scope(
    database: &Arc<dyn Database>,
    search_scope: SearchScope,
    id: &str,
) -> Result<Vec<String>> {
    // For maximizing the performance, we are using a vec of referenced Strings.
    let document_metadata_ids: Vec<String> = match search_scope {
        SearchScope::Userspace => {
            let resource_ids = database.get_resource_ids_by_username(id).await?;
            let collection_metadatas: Vec<CollectionMetadata> = database
                .get_collections(
                    &GetCollectionFilter {
                        ids: resource_ids,
                        ..Default::default()
                    },
                    false,
                )
                .await?;

            collection_metadatas
                .into_iter()
                .flat_map(|item| item.documents_metadatas.into_iter().map(|item| item.id))
                .collect()
        }
        SearchScope::Collection => database
            .get_collections(
                &GetCollectionFilter {
                    ids: vec![id.to_string()],
                    ..Default::default()
                },
                false,
            )
            .await?
            .into_iter()
            .flat_map(|item| item.documents_metadatas.into_iter().map(|item| item.id))
            .collect(),
        SearchScope::Document => vec![id.to_string()],
    };

    Ok(document_metadata_ids)
}

pub fn preprocess_document(
    title: &str,
    content: &str,
    collection_metadata_id: &str,
    chunk_size: usize,
) -> (DocumentMetadata, Vec<DocumentChunk>, String) {
    // Each chunk will relate to one metadata
    let mut metadata: DocumentMetadata =
        DocumentMetadata::new(title.to_string(), collection_metadata_id.to_string());

    // Each chunk can relate to their metadata with a metadata.id
    let chunks: Vec<DocumentChunk> = DocumentChunk::slice_document_automatically(
        content,
        chunk_size,
        &metadata.id,
        collection_metadata_id,
    );

    metadata.chunks = chunks.clone();
    let metadata_id = metadata.id.clone();
    (metadata, chunks, metadata_id)
}
