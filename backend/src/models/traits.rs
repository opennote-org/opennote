use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Verify if the API caller can mutate the data with the input data model
pub trait ValidateDataMutabilitiesForAPICaller {
    /// Throw an error with which data field is immutable to the API caller
    fn is_mutated(&self) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum IndexableField {
    Keyword(String),
    FullText(String),
}

/// Get the fields that will be indexed in the database
/// TODO: make this a macro that will automatically generate the code.
pub trait GetIndexableFields {
    fn get_indexable_fields() -> Vec<IndexableField>;
}
