use std::{collections::HashMap, path::PathBuf};

use actix_web::cookie::time::UtcDateTime;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{
    documents::{
        collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata,
        traits::ValidateDataMutabilitiesForAPICaller,
    },
    traits::LoadAndSave,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataStorage {
    pub path: PathBuf,

    #[serde(default)]
    pub embedder_model_in_use: String,

    #[serde(default)]
    pub embedder_model_vector_size_in_use: usize,

    // key-value pair: collection id, DocumentMetadata
    pub collections: HashMap<String, CollectionMetadata>,

    // key-value pair: document id, DocumentMetadata
    pub documents: HashMap<String, DocumentMetadata>,
}

impl Default for MetadataStorage {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            embedder_model_in_use: String::new(),
            embedder_model_vector_size_in_use: usize::default(),
            documents: HashMap::new(),
            collections: HashMap::new(),
        }
    }
}

impl LoadAndSave for MetadataStorage {
    fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            ..Default::default()
        }
    }

    fn get_path(&self) -> &std::path::Path {
        &self.path
    }
}

impl MetadataStorage {
    pub async fn create_collection(&mut self, title: &str) -> Result<String> {
        let collection: CollectionMetadata = CollectionMetadata::new(title.to_string());
        self.collections
            .insert(collection.metadata_id.clone(), collection.clone());
        let _ = self.save().await?;
        return Ok(collection.metadata_id);
    }

    /// Return None if the given metadata id is not found
    pub async fn delete_collection(
        &mut self,
        collection_metadata_id: &str,
    ) -> Option<CollectionMetadata> {
        let collection_metadata = self.collections.remove(collection_metadata_id);

        if let Some(ref collection_metadata) = collection_metadata {
            self.documents
                .retain(|id, _| !collection_metadata.documents_metadata_ids.contains(&id));
        }

        let _ = self.save().await;
        collection_metadata
    }

    pub async fn update_collection(
        &mut self,
        mut collection_metadatas: Vec<CollectionMetadata>,
    ) -> Result<()> {
        // Same as the document,
        // we perform a non-destructive validation before proceeding into updating.
        for metadata in collection_metadatas.iter_mut() {
            // Verify the documents exist
            if let Some(original_collection_metadata) = self.collections.get(&metadata.metadata_id)
            {
                // Verify if the immutable fields are mutated
                metadata.is_mutated()?;
                // Swap the immutable fields values in
                metadata.documents_metadata_ids =
                    original_collection_metadata.documents_metadata_ids.clone();
                metadata.created_at = original_collection_metadata.created_at.clone();
                metadata.last_modified = UtcDateTime::now().to_string();

                continue;
            }

            return Err(anyhow!(
                "Collection metadata id {} was not found, update operation terminated",
                metadata.metadata_id
            ));
        }

        for metadata in collection_metadatas {
            self.collections
                .insert(metadata.metadata_id.clone(), metadata);
        }

        self.save().await?;

        Ok(())
    }

    /// For updating chunks in document metadatas.
    /// The update method won't allow mutate the chunks for security reason.
    pub async fn update_documents_with_new_chunks(
        &mut self,
        document_metadatas: Vec<DocumentMetadata>,
    ) -> Result<()> {
        for metadata in document_metadatas {
            // Verify the documents exist
            if let Some(_) = self.documents.remove(&metadata.metadata_id) {
                self.documents
                    .insert(metadata.metadata_id.clone(), metadata.to_owned());
                continue;
            }

            return Err(anyhow!(
                "Document metadata id {} was not found, update operation terminated",
                metadata.metadata_id
            ));
        }

        self.save().await?;
        Ok(())
    }

    pub async fn update_documents(
        &mut self,
        mut document_metadatas: Vec<DocumentMetadata>,
    ) -> Result<()> {
        // Perform a non-destructive verification before proceeding into updating.
        // This ensures the update operation itself won't be errored out,
        // and endup breaking the metadata storage.
        for metadata in document_metadatas.iter_mut() {
            // Verify the documents exist
            if let Some(original_document_metadata) = self.documents.get(&metadata.metadata_id) {
                // Verify if the immutable fields are mutated
                metadata.is_mutated()?;
                // Swap the immutable fields values in
                metadata.chunks = original_document_metadata.chunks.clone();
                metadata.created_at = original_document_metadata.created_at.clone();
                metadata.last_modified = UtcDateTime::now().to_string();

                // If the document is being moved to another collection, verify the destination collection exists
                if metadata.collection_metadata_id
                    != original_document_metadata.collection_metadata_id
                {
                    if !self
                        .collections
                        .contains_key(&metadata.collection_metadata_id)
                    {
                        return Err(anyhow!(
                            "Target collection id {} was not found, update operation terminated",
                            metadata.collection_metadata_id
                        ));
                    }
                }

                continue;
            }

            return Err(anyhow!(
                "Document metadata id {} was not found, update operation terminated",
                metadata.metadata_id
            ));
        }

        for metadata in document_metadatas {
            // If the document has changed its belonging collection,
            // we need to update accordingly, otherwise it will be wrong
            if let Some(old_document_metadata) = self.documents.remove(&metadata.metadata_id) {
                if metadata.collection_metadata_id != old_document_metadata.collection_metadata_id {
                    // Remove the document id from the original collection first,
                    // before we update the destiny collection.
                    if let Some(original_collection_metadata) = self
                        .collections
                        .get_mut(&old_document_metadata.collection_metadata_id)
                    {
                        original_collection_metadata
                            .documents_metadata_ids
                            .retain(|id| *id != metadata.metadata_id);
                    }

                    // Now we update the destiny collection
                    if let Some(destiny_collection_metadata) =
                        self.collections.get_mut(&metadata.collection_metadata_id)
                    {
                        destiny_collection_metadata
                            .documents_metadata_ids
                            .push(metadata.metadata_id.clone());
                    }
                }

                // If, by any chance, nothing was removed,
                // we don't do insert operations to protect the data consistency.
                self.documents
                    .insert(metadata.metadata_id.clone(), metadata);
            }
        }

        self.save().await?;
        Ok(())
    }

    pub async fn add_document(&mut self, metadata: DocumentMetadata) -> Result<()> {
        if let Some(collection) = self.collections.get_mut(&metadata.collection_metadata_id) {
            collection
                .documents_metadata_ids
                .push(metadata.metadata_id.clone());
            self.documents
                .insert(metadata.metadata_id.clone(), metadata);
            let _ = self.save().await?;
            return Ok(());
        }

        Err(anyhow!(
            "Collection {} is missing. Please create a collection before adding new documents to it",
            metadata.collection_metadata_id
        ))
    }
    
    pub async fn remove_document(&mut self, metdata_id: &str) -> Option<DocumentMetadata> {
        let document_metadata: Option<DocumentMetadata> = self.documents.remove(metdata_id);

        if let Some(ref document_metadata) = document_metadata {
            if let Some(collection) = self
                .collections
                .get_mut(&document_metadata.collection_metadata_id)
            {
                let mut index_to_remove = None;
                for (index, id) in collection.documents_metadata_ids.iter().enumerate() {
                    if id == metdata_id {
                        index_to_remove = Some(index);
                    }
                }

                if let Some(index) = index_to_remove {
                    collection.documents_metadata_ids.remove(index);
                }
            }
        }

        let _ = self.save().await;

        document_metadata
    }

    pub fn get_document_ids_by_collection(&self, collection_metadata_id: &str) -> Vec<&String> {
        if let Some(collection_metadata) = self.collections.get(collection_metadata_id) {
            return collection_metadata
                .documents_metadata_ids
                .iter()
                .map(|document_metadata_id| document_metadata_id)
                .collect();
        }

        Vec::new()
    }
}
