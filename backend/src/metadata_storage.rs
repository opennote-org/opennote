use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    documents::{
        collection_metadata::CollectionMetadata, document_metadata::DocumentMetadata,
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
    pub fn is_metadata_storage_exist(path: &str) -> bool {
        match std::fs::exists(path) {
            Ok(result) => result,
            Err(_) => false,
        }
    }
}