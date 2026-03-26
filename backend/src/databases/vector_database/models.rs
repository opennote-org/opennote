use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDatabasePayload {
    /// For retrieving corresponding payload in relational database
    pub correspondent_id: Uuid,
    /// For vector search
    pub vector: Vec<f32>,
    /// For full text search 
    pub texts: String,
}
