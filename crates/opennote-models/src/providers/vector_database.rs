use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorDatabaseProvider {
    LanceDB,
    SQLiteVector,
}

impl Display for VectorDatabaseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LanceDB => f.write_str("lancedb"),
            Self::SQLiteVector => f.write_str("sqlite-vector"),
        }
    }
}
