use crate::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetUserFilter {
    pub id: Option<String>,
    pub username: Option<String>,
    pub resources: Option<Vec<String>>,
}

impl GetFilterValidation for GetUserFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            self.id.is_some(),
            self.username.is_some(),
            self.resources.is_some(),
        ]
    }
}

impl Default for GetUserFilter {
    fn default() -> Self {
        Self {
            id: None,
            username: None,
            resources: None,
        }
    }
}
