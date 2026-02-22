use crate::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetCollectionFilter {
    pub id: Option<String>,

    pub created_at: Option<String>,

    pub last_modified: Option<String>,

    pub title: Option<String>,

    // metadata ids of its owned documents
    pub documents_metadata_ids: Option<Vec<String>>,
}

impl GetFilterValidation for GetCollectionFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            self.id.is_some(),
            self.created_at.is_some(),
            self.last_modified.is_some(),
            self.title.is_some(),
            self.documents_metadata_ids.is_some(),
        ]
    }
}