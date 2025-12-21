//! This is the fundamental data structure of the notebook app.

use std::collections::HashMap;

use jieba_rs::Jieba;
use qdrant_client::{
    Payload,
    qdrant::{NamedVectors, PointStruct, RetrievedPoint, ScoredPoint},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

    /// Split the sentences to prevent token numbers exceeds the model limit
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
                sentences.push(sentence);
                break;
            }
        }

        sentences
    }

    pub fn slice_document_by_period(
        content: &str,
        chunk_max_words: usize,
        document_metadata_id: &str,
        collection_metadata_id: &str,
    ) -> Vec<DocumentChunk> {
        let jieba = Jieba::new();
        let mut chunks: Vec<DocumentChunk> = Vec::new();
        let terminator: char = if content.contains('。') { '。' } else { '.' };

        for sentence in content.split(terminator) {
            let words: Vec<&str> = jieba.cut(sentence, false);

            if words.len() > chunk_max_words {
                let sentences = Self::split_by_n(words, chunk_max_words);
                for sentence in sentences {
                    let mut to_push: String = sentence;
                    to_push.push(terminator);

                    chunks.push(DocumentChunk::new(
                        to_push,
                        document_metadata_id,
                        collection_metadata_id,
                    ));
                }

                continue;
            }

            let mut to_push: String = sentence.to_string();
            to_push.push(terminator);

            chunks.push(DocumentChunk::new(
                to_push,
                document_metadata_id,
                collection_metadata_id,
            ));
        }

        chunks
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
                    }
                )
            ,
            Payload::try_from(serde_json::to_value(value).unwrap()).unwrap(),
        )
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentChunkSearchResult {
    pub document_chunk: DocumentChunk,
    /// Similarity score
    pub score: f32,
}

impl From<RetrievedPoint> for DocumentChunkSearchResult {
    fn from(value: RetrievedPoint) -> Self {
        Self { 
            document_chunk: value.into(), 
            score: 0.0 
        }
    }
}

impl From<ScoredPoint> for DocumentChunkSearchResult {
    fn from(value: ScoredPoint) -> Self {
        Self {
            document_chunk: DocumentChunk::from(value.payload),
            score: value.score,
        }
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
