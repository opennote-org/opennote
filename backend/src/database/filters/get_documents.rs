use crate::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetDocumentFilter {
    pub ids: Vec<String>,

    pub created_at: Option<String>,

    pub last_modified: Option<String>,

    pub title: Option<String>,

    pub collection_metadata_ids: Vec<String>,
}

impl GetFilterValidation for GetDocumentFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            !self.ids.is_empty(),
            !self.collection_metadata_ids.is_empty(),
            self.created_at.is_some(),
            self.last_modified.is_some(),
            self.title.is_some(),
        ]
    }
}

impl Default for GetDocumentFilter {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            created_at: None,
            last_modified: None,
            title: None,
            collection_metadata_ids: Vec::new(),
        }
    }
}
