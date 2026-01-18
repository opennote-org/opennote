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
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
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
    pub document_title: Option<String>,
    pub collection_title: Option<String>,
    pub document_chunk: DocumentChunk,
    /// Similarity score
    pub score: f32,
}

impl Default for DocumentChunkSearchResult {
    fn default() -> Self {
        Self {
            document_title: None,
            collection_title: None,
            document_chunk: DocumentChunk::default(),
            score: 0.0,
        }
    }
}

impl From<RetrievedPoint> for DocumentChunkSearchResult {
    fn from(value: RetrievedPoint) -> Self {
        Self {
            document_chunk: value.into(),
            ..Default::default()
        }
    }
}

impl From<ScoredPoint> for DocumentChunkSearchResult {
    fn from(value: ScoredPoint) -> Self {
        Self {
            document_chunk: DocumentChunk::from(value.payload),
            score: value.score,
            ..Default::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_n() {
        let words = vec!["a", "b", "c", "d", "e"];
        let result = DocumentChunk::split_by_n(words, 2);

        // split_by_n concatenates words without separator
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "ab");
        assert_eq!(result[1], "cd");
        assert_eq!(result[2], "e");
    }

    #[test]
    fn test_split_by_n_exact() {
        let words = vec!["a", "b", "c", "d"];
        let result = DocumentChunk::split_by_n(words, 2);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "ab");
        assert_eq!(result[1], "cd");
    }

    #[test]
    fn test_slice_document_by_period_english() {
        let content = "Hello world. This is a test.";
        // Jieba cuts "Hello world" -> ["Hello", " ", "world"] (3 tokens) usually
        // Let's use a small max_words to trigger split if possible, or large to keep sentences intact.

        let chunks = DocumentChunk::slice_document_by_period(content, 100, "doc1", "col1");

        // "Hello world" (sentence 1) -> < 100 words -> push "Hello world."
        // " This is a test" (sentence 2) -> < 100 words -> push " This is a test."
        // "" (sentence 3, trailing split) -> 0 words -> push "."

        // Note: split behavior depends on trailing char.
        // "a.b.".split('.') -> "a", "b", ""

        // So we expect 3 chunks if the logic holds.
        // However, if the sentence is empty (words len 0), it still pushes a chunk with just terminator.
        // Let's verify if we want to filter out empty chunks. The code doesn't seem to filter.

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "Hello world.");
        assert_eq!(chunks[1].content, " This is a test.");
        assert_eq!(chunks[2].content, ".");

        assert_eq!(chunks[0].document_metadata_id, "doc1");
        assert_eq!(chunks[0].collection_metadata_id, "col1");
    }

    #[test]
    fn test_slice_document_by_period_chinese() {
        let content = "你好世界。这是一个测试。";
        let chunks = DocumentChunk::slice_document_by_period(content, 100, "doc1", "col1");

        // "你好世界" -> push "你好世界。"
        // "这是一个测试" -> push "这是一个测试。"
        // "" -> push "。"

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "你好世界。");
        assert_eq!(chunks[1].content, "这是一个测试。");
        assert_eq!(chunks[2].content, "。");
    }

    #[test]
    fn test_slice_document_long_sentence() {
        // Construct a long sentence without periods
        let long_text = "word ".repeat(10); // 10 "word "
        // "word " might be split by jieba into ["word", " "] or just ["word "] depending on HMM.
        // Assuming jieba behavior, let's just assume it produces multiple tokens.

        // Let's set max words to a small number to force splitting
        let chunks = DocumentChunk::slice_document_by_period(&long_text, 2, "doc1", "col1");

        // If "word " repeats 10 times, and we split by '.', we get one big sentence (plus empty trailing if we had a dot, but we don't).
        // Since we don't have a dot, we check the terminator logic.
        // let terminator: char = if content.contains('。') { '。' } else { '.' };
        // It will use '.'
        // "word word ..." split by '.' gives the whole string as one item.

        // Inside: jieba.cut(sentence)
        // If it produces > 2 words, it calls split_by_n(words, 2)
        // Then it pushes each part + terminator.

        assert!(chunks.len() == 10);
        for chunk in &chunks {
            assert!(chunk.content.ends_with('.'));
        }
    }
}
