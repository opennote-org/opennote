#[derive(Debug, Clone)]
pub struct GetUserFilter {
    pub id: Option<String>,
    pub username: Option<String>,
    pub resources: Option<Vec<String>>,
}

impl GetUserFilter {
    pub fn is_over_constrained(&self) -> bool {
        let parameters = [
            self.id.is_some(),
            self.username.is_some(),
            self.resources.is_some(),
        ]
        .iter()
        .filter(|item| **item)
        .count();

        parameters > 1
    }
}
