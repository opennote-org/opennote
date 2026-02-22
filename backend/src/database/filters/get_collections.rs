use crate::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetCollectionFilter {
    pub id: Option<String>,

    pub created_at: Option<String>,

    pub last_modified: Option<String>,

    pub title: Option<String>,
}

impl GetFilterValidation for GetCollectionFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            self.id.is_some(),
            self.created_at.is_some(),
            self.last_modified.is_some(),
            self.title.is_some(),
        ]
    }
}

impl Default for GetCollectionFilter {
    fn default() -> Self {
        Self {
            id: None,
            created_at: None,
            last_modified: None,
            title: None,
        }
    }
}
