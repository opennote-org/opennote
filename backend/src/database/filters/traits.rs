pub trait GetFilterValidation {
    /// Implement this to get a free `is_over_constrained` method
    /// return the number of Some elements in the implemented struct's field
    /// Refer to `src/database/filters/get_users.rs` for an example
    fn get_num_some(&self) -> Vec<bool>;

    fn is_over_constrained(&self) -> bool {
        let parameters: usize = self.get_num_some().iter().filter(|item| **item).count();
        parameters > 1
    }
}
