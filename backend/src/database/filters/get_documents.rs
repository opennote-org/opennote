use crate::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetDocumentFilter {
    pub id: Option<String>,

    pub created_at: Option<String>,

    pub last_modified: Option<String>,

    pub title: Option<String>,

    pub collection_metadata_id: Option<String>,
}

impl GetFilterValidation for GetDocumentFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            self.id.is_some(),
            self.created_at.is_some(),
            self.last_modified.is_some(),
            self.title.is_some(),
            self.collection_metadata_id.is_some(),
        ]
    }
}

impl Default for GetDocumentFilter {
    fn default() -> Self {
        Self {
            id: None,
            created_at: None,
            last_modified: None,
            title: None,
            collection_metadata_id: None,
        }
    }
}
