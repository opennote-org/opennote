use crate::databases::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetDocumentChunkFilter {
    pub ids: Vec<String>,
    pub document_metadata_ids: Vec<String>,
    pub collection_metadata_ids: Vec<String>,
}

impl GetFilterValidation for GetDocumentChunkFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            !self.ids.is_empty(),
            !self.document_metadata_ids.is_empty(),
            !self.collection_metadata_ids.is_empty(),
        ]
    }
}

impl Default for GetDocumentChunkFilter {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            document_metadata_ids: Vec::new(),
            collection_metadata_ids: Vec::new(),
        }
    }
}
