use crate::databases::database::filters::traits::GetFilterValidation;

#[derive(Debug, Clone)]
pub struct GetUserFilter {
    pub ids: Vec<String>,
    pub usernames: Vec<String>,
}

impl GetFilterValidation for GetUserFilter {
    fn get_num_some(&self) -> Vec<bool> {
        vec![
            !self.ids.is_empty(),
            !self.usernames.is_empty(),
        ]
    }
}

impl Default for GetUserFilter {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            usernames: Vec::new(),
        }
    }
}
