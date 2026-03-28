use serde::{Deserialize, Serialize};

use crate::connectors::requests::ImportTask;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportDocumentsResponse {
    pub failed_import_tasks: Vec<ImportTask>,
    pub document_metadata_ids: Vec<String>,
}
