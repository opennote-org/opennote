//! This is the fundamental data structure of the notebook app.

use std::{collections::HashMap, io::Read};

use chunk::chunk;
use jieba_rs::Jieba;
use qdrant_client::{
    Payload,
    qdrant::{NamedVectors, PointStruct, RetrievedPoint},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{databases::database, documents::traits::GetId};

use super::traits::{GetIndexableFields, IndexableField};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct DocumentChunk {
    pub id: String,
    pub document_metadata_id: String,
    pub collection_metadata_id: String,
    pub content: String,
    #[serde(skip)]
    pub dense_text_vector: Vec<f32>,
}

impl Default for DocumentChunk {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            document_metadata_id: "".to_string(),
            collection_metadata_id: "".to_string(),
            content: "".to_string(),
            dense_text_vector: vec![],
        }
    }
}

impl DocumentChunk {
    pub fn new(content: String, document_metadata_id: &str, collection_metadata_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            document_metadata_id: document_metadata_id.to_string(),
            collection_metadata_id: collection_metadata_id.to_string(),
            dense_text_vector: Vec::new(),
        }
    }

    pub fn slice_document_automatically(
        content: &str,
        chunk_max_words: usize,
        document_metadata_id: &str,
        collection_metadata_id: &str,
    ) -> Vec<DocumentChunk> {
        let mut chunks: Vec<DocumentChunk> = Vec::new();

        let raw_chunks: Vec<_> = chunk(content.as_bytes())
            .consecutive()
            .delimiters("\n.?!。，！".as_bytes())
            .size(chunk_max_words)
            .collect();

        for mut chunk in raw_chunks {
            let mut bytes = Vec::new();
            match chunk.read_to_end(&mut bytes) {
                Ok(_) => {
                    chunks.push(DocumentChunk::new(
                        String::from_utf8_lossy(&bytes).to_string(),
                        document_metadata_id,
                        collection_metadata_id,
                    ));
                }
                Err(error) => {
                    log::warn!("Error reading chunk: {} Chunk content: {:?}", error, chunk);
                }
            }
        }

        chunks
    }

    /// Split the sentences to prevent token numbers exceeds the model limit
    /// To be DEPRECATED, as `slice_document_automatically` stablizes
    #[allow(dead_code)]
    fn split_by_n(words: Vec<&str>, chunk_max_word_length: usize) -> Vec<String> {
        let mut remaining: Vec<&str> = words;
        let mut sentences: Vec<String> = Vec::new();

        let mut sentence = String::new();
        let mut count = 0;
        while !remaining.is_empty() {
            let first = remaining.remove(0);
            sentence.push_str(first);
            count += 1;

            if count == chunk_max_word_length {
                count = 0;
                sentences.push(sentence);
                sentence = String::new();
            }

            if remaining.is_empty() {
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                break;
            }
        }

        sentences
    }

    /// To be DEPRECATED, as `slice_document_automatically` stablizes
    #[allow(dead_code)]
    #[deprecated]
    pub fn slice_document_by_period(
        content: &str,
        chunk_max_words: usize,
        document_metadata_id: &str,
        collection_metadata_id: &str,
    ) -> Vec<DocumentChunk> {
        let jieba = Jieba::new();
        let mut chunks: Vec<DocumentChunk> = Vec::new();
        let terminator: char = if content.contains('。') { '。' } else { '.' };

        // Using inclusive split to avoid chopping off the periods at the end
        for sentence in content.split_inclusive(terminator) {
            let words: Vec<&str> = jieba.cut(sentence, false);

            if words.len() > chunk_max_words {
                let sentences_split_by_n = Self::split_by_n(words, chunk_max_words);
                for sentence_split_by_n in sentences_split_by_n {
                    let to_push: String = sentence_split_by_n;

                    chunks.push(DocumentChunk::new(
                        to_push,
                        document_metadata_id,
                        collection_metadata_id,
                    ));
                }

                continue;
            }

            chunks.push(DocumentChunk::new(
                sentence.to_string(),
                document_metadata_id,
                collection_metadata_id,
            ));
        }

        chunks
    }
}

impl GetId for DocumentChunk {
    fn get_id(&self) -> &str {
        &self.id
    }
}

impl From<DocumentChunk> for PointStruct {
    fn from(value: DocumentChunk) -> Self {
        Self::new(
            value.id.clone(),
            NamedVectors::default()
                .add_vector("dense_text_vector", value.dense_text_vector.clone())
                .add_vector(
                    "sparse_text_vector",
                    qdrant_client::qdrant::Document {
                        text: value.content.clone(),
                        model: "qdrant/bm25".into(),
                        ..Default::default()
                    },
                ),
            Payload::try_from(serde_json::to_value(value).unwrap()).unwrap(),
        )
    }
}

impl From<HashMap<String, serde_json::Value>> for DocumentChunk {
    fn from(value: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: value.get("id").unwrap().as_str().unwrap().to_string(),
            content: value.get("content").unwrap().as_str().unwrap().to_string(),
            document_metadata_id: value
                .get("document_metadata_id")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            collection_metadata_id: value
                .get("collection_metadata_id")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            dense_text_vector: vec![],
        }
    }
}

impl From<HashMap<String, qdrant_client::qdrant::Value>> for DocumentChunk {
    fn from(value: HashMap<String, qdrant_client::qdrant::Value>) -> Self {
        Self {
            id: value.get("id").unwrap().as_str().unwrap().to_string(),
            content: value.get("content").unwrap().as_str().unwrap().to_string(),
            document_metadata_id: value
                .get("document_metadata_id")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            collection_metadata_id: value
                .get("collection_metadata_id")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            dense_text_vector: vec![],
        }
    }
}

impl From<RetrievedPoint> for DocumentChunk {
    fn from(value: RetrievedPoint) -> Self {
        DocumentChunk::from(value.payload)
    }
}

impl GetIndexableFields for DocumentChunk {
    fn get_indexable_fields() -> Vec<IndexableField> {
        vec![
            IndexableField::FullText("content".to_string()),
            IndexableField::Keyword("document_metadata_id".to_string()),
            IndexableField::Keyword("collection_metadata_id".to_string()),
            IndexableField::Keyword("id".to_string()),
        ]
    }
}

impl From<database::entity::document_chunks::Model> for DocumentChunk {
    fn from(value: database::entity::document_chunks::Model) -> Self {
        Self {
            id: value.id,
            document_metadata_id: value.document_metadata_id,
            collection_metadata_id: value.collection_metadata_id,
            content: value.content,
            dense_text_vector: serde_json::from_value(value.dense_text_vector).unwrap(),
        }
    }
}

impl From<DocumentChunk> for database::entity::document_chunks::Model {
    fn from(value: DocumentChunk) -> Self {
        Self {
            id: value.id,
            document_metadata_id: value.document_metadata_id,
            collection_metadata_id: value.collection_metadata_id,
            content: value.content,
            dense_text_vector: serde_json::to_value(value.dense_text_vector).unwrap(),
            chunk_order: 0,
        }
    }
}
