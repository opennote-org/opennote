#![forbid(unsafe_code)]

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Constants used for special field names
pub mod constants {
    /// Identifier field name
    pub const F_ID: &str = "__id__";
    /// Similarity metrics field name
    pub const F_METRICS: &str = "__metrics__";
}

type Float = f32;

/// A single vector entry with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    /// Unique identifier for the vector
    #[serde(rename = "__id__")]
    pub id: String,
    /// The vector data (non-normalized)
    #[serde(skip)]
    pub vector: Vec<Float>,
    /// Additional metadata fields stored with the vector
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataBase {
    embedding_dim: usize,
    
    data: Vec<Data>,
    
    #[serde(with = "base64_bytes")]
    matrix: Vec<Float>,
    
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    additional_data: HashMap<String, serde_json::Value>,
    
    keywords_index: HashMap<String, Vec<String>>,
}

mod base64_bytes {
    use super::*;
    use bytemuck::cast_slice;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(vec: &[Float], serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = cast_slice(vec);
        let b64 = general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&b64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Float>, D::Error> {
        let s = String::deserialize(deserializer)?;
        let bytes = general_purpose::STANDARD
            .decode(s)
            .map_err(serde::de::Error::custom)?;
        Ok(bytes
            .chunks_exact(4)
            .map(|chunk| Float::from_le_bytes(chunk.try_into().unwrap()))
            .collect())
    }
}

/// Main vector database struct
#[derive(Debug)]
pub struct LocalVectorDatabase {
    /// Dimensionality of stored vectors
    pub embedding_dim: usize,
    /// Distance metric used for similarity searches
    pub metric: String,
    storage_file: PathBuf,
    storage: DataBase,
}

#[derive(PartialEq)]
struct ScoredIndex {
    score: Float,
    index: usize,
}

impl Eq for ScoredIndex {}

impl PartialOrd for ScoredIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        other.score.partial_cmp(&self.score).unwrap_or_else(|| {
            if self.score.is_nan() && other.score.is_nan() {
                Ordering::Equal
            } else if self.score.is_nan() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        })
    }
}

type DataFilter = Box<dyn Fn(&Data) -> bool + Send + Sync>;

impl LocalVectorDatabase {
    /// Creates a new LocalVectorDatabase instance
    pub fn new(embedding_dim: usize, storage_file: &str) -> Result<Self> {
        let storage_file = PathBuf::from(storage_file);
        let storage = if storage_file.exists() && storage_file.metadata()?.len() > 0 {
            let contents = fs::read_to_string(&storage_file)?;
            let db: DataBase = serde_json::from_str(&contents)?;

            let expected_len = db.data.len() * db.embedding_dim;
            if db.matrix.len() != expected_len {
                anyhow::bail!(
                    "Matrix size mismatch: expected {}, got {}",
                    expected_len,
                    db.matrix.len()
                );
            }

            db
        } else {
            DataBase {
                embedding_dim,
                data: Vec::new(),
                matrix: Vec::new(),
                additional_data: HashMap::new(),
                keywords_index: HashMap::new(),
            }
        };

        Ok(Self {
            embedding_dim,
            metric: "cosine".to_string(),
            storage_file,
            storage,
        })
    }

    /// Upserts vectors into the database
    pub fn upsert(&mut self, mut datas: Vec<Data>) -> Result<(Vec<String>, Vec<String>)> {
        let mut updates = Vec::new();
        let mut inserts = Vec::new();
        let existing_ids: HashSet<_> = self.storage.data.iter().map(|d| &d.id).collect();
        
        // 1. Extract keywords out of the data
        // 2. update the index hashmap
        
        // Alternative approach to Rewrite vector database
        // store chunks info in the backend
        // only preserve id references to the chunks in the vector database
        // user could recover the vector database from the backend
        // 
        // this also allows making the keyword search a separate module aside from the vector databases

        for data in datas.iter_mut() {
            if existing_ids.contains(&data.id) {
                if let Some(pos) = self.storage.data.iter().position(|d| d.id == data.id) {
                    let norm_vec = normalize(&data.vector);
                    let start = pos * self.embedding_dim;
                    let end = start + self.embedding_dim;
                    self.storage.matrix[start..end].copy_from_slice(&norm_vec);
                    updates.push(data.id.clone());
                }
            }
        }

        let new_datas: Vec<Data> = datas
            .into_iter()
            .filter(|d| !existing_ids.contains(&d.id))
            .collect();

        for data in new_datas {
            let norm_vec = normalize(&data.vector);
            let vec_clone = norm_vec.clone();
            self.storage.matrix.extend(vec_clone);
            self.storage.data.push(Data {
                id: data.id.clone(),
                vector: norm_vec,
                fields: data.fields,
            });
            inserts.push(data.id);
        }

        Ok((updates, inserts))
    }
    
    /// Queries the database with keywords
    pub fn query_keywords(&self, query: &str) -> Vec<HashMap<String, serde_json::Value>> {
        vec![]
    }

    /// Queries the database for similar vectors
    pub fn query(
        &self,
        query: &[Float],
        top_k: usize,
        better_than: Option<Float>,
        filter: Option<DataFilter>,
    ) -> Vec<HashMap<String, serde_json::Value>> {
        let query_norm = normalize(query);
        let embedding_dim = self.embedding_dim;
        let matrix = &self.storage.matrix;
        let threshold = better_than.unwrap_or(Float::MIN);

        // Precompute query chunks for SIMD-friendly operations
        let query_chunks: Vec<[Float; 4]> = query_norm
            .chunks_exact(4)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
            .collect();
        let query_remainder = &query_norm[query_chunks.len() * 4..];

        // Parallel processing with Rayon
        let heap = matrix
            .par_chunks(embedding_dim)
            .enumerate()
            .filter(|(idx, _)| {
                filter
                    .as_ref()
                    .map(|f| f(&self.storage.data[*idx]))
                    .unwrap_or(true)
            })
            .fold(
                || BinaryHeap::with_capacity(top_k + 1),
                |mut heap, (idx, vector)| {
                    let score = dot_product(vector, &query_chunks, query_remainder);

                    if score >= threshold {
                        heap.push(ScoredIndex { score, index: idx });
                        if heap.len() > top_k {
                            heap.pop();
                        }
                    }
                    heap
                },
            )
            .reduce(
                || BinaryHeap::with_capacity(top_k + 1),
                |mut heap1, heap2| {
                    for si in heap2 {
                        heap1.push(si);
                        if heap1.len() > top_k {
                            heap1.pop();
                        }
                    }
                    heap1
                },
            );

        // Convert to sorted results
        let sorted = heap.into_sorted_vec();

        sorted
            .into_iter()
            .map(|si| {
                let data = &self.storage.data[si.index];
                let mut result = data.fields.clone();
                result.insert(
                    constants::F_METRICS.to_string(),
                    serde_json::json!(si.score),
                );
                result.insert(constants::F_ID.to_string(), serde_json::json!(data.id));
                result
            })
            .collect()
    }

    /// Get vectors by their IDs
    pub fn get(&self, ids: &[String]) -> Vec<&Data> {
        let id_set: HashSet<_> = ids.iter().collect();
        self.storage
            .data
            .iter()
            .filter(|data| id_set.contains(&data.id))
            .collect()
    }
    
    /// Get all vectors in the database as owned
    pub fn get_all_owned(&self) -> Vec<Data> {
        self.storage.data.clone()
    }
    
    /// Get all vectors in the database as a reference
    pub fn get_all(&self) -> &Vec<Data> {
        &self.storage.data
    }

    /// Delete vectors by their IDs
    pub fn delete(&mut self, ids: &[String]) {
        let id_set: HashSet<_> = ids.iter().collect();

        // Filter out deleted entries
        self.storage.data.retain(|data| !id_set.contains(&data.id));

        // Rebuild matrix from remaining vectors
        self.storage.matrix = self
            .storage
            .data
            .iter()
            .flat_map(|data| data.vector.iter().copied())
            .collect();
    }

    /// Saves the database to disk
    pub fn save(&self) -> Result<()> {
        let serialized = serde_json::to_string(&self.storage)?;
        fs::write(&self.storage_file, serialized)?;
        Ok(())
    }

    /// Get additional metadata stored in the database
    pub fn get_additional_data(&self) -> &HashMap<String, serde_json::Value> {
        &self.storage.additional_data
    }

    /// Store additional metadata in the database
    pub fn store_additional_data(&mut self, data: HashMap<String, serde_json::Value>) {
        self.storage.additional_data = data;
    }

    /// Get the number of vectors in the database
    pub fn len(&self) -> usize {
        self.storage.data.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.storage.data.is_empty()
    }

    /// Get total vector bytes length
    pub fn vector_bytes_len(&self) -> usize {
        self.storage.matrix.len()
    }
}

#[inline]
/// Calculate the dot product between two vectors
pub fn dot_product(vec: &[Float], query_chunks: &[[Float; 4]], query_remainder: &[Float]) -> Float {
    assert_eq!(
        query_chunks.len() * 4 + query_remainder.len(),
        vec.len(),
        "Mismatched lengths between vector and query components"
    );

    let sum = vec
        .chunks_exact(4)
        .zip(query_chunks)
        .fold(0.0, |acc, (chunk, q)| {
            acc + chunk.iter().zip(q).map(|(a, b)| a * b).sum::<Float>()
        });

    sum + vec
        .chunks_exact(4)
        .remainder()
        .iter()
        .zip(query_remainder)
        .map(|(a, b)| a * b)
        .sum::<Float>()
}

/// Normalize a vector to unit length
pub fn normalize(vector: &[Float]) -> Vec<Float> {
    let norm_sq: Float = vector
        .iter()
        .fold(0.0 as Float, |acc, &x| x.mul_add(x, acc));

    assert!(
        norm_sq > Float::EPSILON,
        "Cannot normalize zero-length vector"
    );

    let inv_norm = 1.0 / norm_sq.sqrt();
    vector.iter().map(|&x| x * inv_norm).collect()
}

/// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    #[test]
    fn test_base64_deserialization_edge_cases() {
        // Test valid base64 deserialization
        let valid_db = DataBase {
            embedding_dim: 2,
            data: vec![Data {
                id: "test".to_string(),
                vector: vec![1.0, 2.0],
                fields: HashMap::new(),
            }],
            matrix: vec![1.0, 2.0],
            additional_data: HashMap::new(),
        };
        let serialized = serde_json::to_string(&valid_db).unwrap();
        let deserialized: DataBase = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.matrix, vec![1.0, 2.0]);

        // Test invalid base64 string
        let invalid_json = r#"{
            "embedding_dim": 2,
            "data": [{"__id__": "test", "vector": [], "fields": {}}],
            "matrix": "INVALID_BASE64!!",
            "additional_data": {}
        }"#;
        let result: Result<DataBase, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_matrix_size_validation() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Create malformed database with mismatched matrix size
        let corrupt_db = DataBase {
            embedding_dim: 2,
            data: vec![Data {
                id: "bad_entry".to_string(),
                vector: vec![1.0, 2.0], // Valid 2D vector
                fields: HashMap::new(),
            }],
            matrix: vec![1.0], // Should be 2 elements for 2D embedding
            additional_data: HashMap::new(),
        };

        // Write corrupted data to file
        fs::write(path, serde_json::to_string(&corrupt_db).unwrap()).unwrap();

        // Attempt to load with validation
        let result = LocalVectorDatabase::new(2, path);

        // Verify error handling
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Matrix size mismatch"));
        assert!(err_msg.contains("expected 2"));
        assert!(err_msg.contains("got 1"));
    }

    #[test]
    fn test_scored_index_ordering() {
        let cases = vec![
            (
                ScoredIndex {
                    score: 0.9,
                    index: 0,
                },
                ScoredIndex {
                    score: 0.8,
                    index: 1,
                },
                Ordering::Less,
            ),
            (
                ScoredIndex {
                    score: 0.5,
                    index: 0,
                },
                ScoredIndex {
                    score: 0.5,
                    index: 1,
                },
                Ordering::Equal,
            ),
            (
                ScoredIndex {
                    score: 0.3,
                    index: 0,
                },
                ScoredIndex {
                    score: 0.4,
                    index: 1,
                },
                Ordering::Greater,
            ),
            // NaN cases
            (
                ScoredIndex {
                    score: f32::NAN,
                    index: 0,
                },
                ScoredIndex {
                    score: 0.5,
                    index: 1,
                },
                Ordering::Less,
            ),
            (
                ScoredIndex {
                    score: 0.5,
                    index: 0,
                },
                ScoredIndex {
                    score: f32::NAN,
                    index: 1,
                },
                Ordering::Greater,
            ),
            (
                ScoredIndex {
                    score: f32::NAN,
                    index: 0,
                },
                ScoredIndex {
                    score: f32::NAN,
                    index: 1,
                },
                Ordering::Equal,
            ),
        ];

        for (a, b, expected) in cases {
            assert_eq!(a.cmp(&b), expected);
        }
    }
}
