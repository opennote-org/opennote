use tokio::sync::MutexGuard;

use crate::{
    documents::{document_chunk::DocumentChunk, document_metadata::DocumentMetadata},
    identities::storage::IdentitiesStorage,
    metadata_storage::MetadataStorage,
    search::SearchScope,
};

pub fn retrieve_document_ids_by_scope(
    metadata_storage: &mut MutexGuard<'_, MetadataStorage>,
    identities_storage: &mut MutexGuard<'_, IdentitiesStorage>,
    search_scope: SearchScope,
    id: &str,
) -> Vec<String> {
    // For maximizing the performance, we are using a vec of referenced Strings.
    let document_metadata_ids: Vec<String> = match search_scope {
        SearchScope::Userspace => {
            let collection_ids: Vec<&String> = identities_storage.get_resource_ids_by_username(id);
            let mut document_metadata_ids: Vec<&String> = Vec::new();

            for id in collection_ids {
                document_metadata_ids.extend(metadata_storage.get_document_ids_by_collection(id));
            }

            document_metadata_ids
                .into_iter()
                .map(|item| item.to_string())
                .collect()
        }
        SearchScope::Collection => metadata_storage
            .get_document_ids_by_collection(id)
            .into_iter()
            .map(|item| item.to_string())
            .collect(),
        SearchScope::Document => vec![id.to_string()],
    };

    document_metadata_ids
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

    metadata.chunks = chunks.iter().map(|chunk| chunk.id.clone()).collect();
    let metadata_id = metadata.id.clone();
    (metadata, chunks, metadata_id)
}
