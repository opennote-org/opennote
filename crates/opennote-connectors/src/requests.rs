use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum ImportType {
    Webpage,
    TextFile,
    RelationshipDatabase,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct ImportTask {
    pub import_type: ImportType,
    pub artifact: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportDocumentsRequest {
    pub username: String,
    pub collection_metadata_id: String,
    pub imports: Vec<ImportTask>,
}
