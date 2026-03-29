use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseProvider {
    #[serde(rename = "sqlite")]
    SQLite,
}