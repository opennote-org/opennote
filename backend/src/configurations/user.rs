//! This file defines configurations that are modifiable by individual users.
//! These are not necessarily break changes to the global uses, 
//! but will directly affect the user him/herself. 

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct UserConfigurations {
    pub search: UserSearchConfiguration,
}

impl Default for UserConfigurations {
    fn default() -> Self {
        Self { search: UserSearchConfiguration::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct UserSearchConfiguration {
    pub document_chunk_size: usize,
}

impl Default for UserSearchConfiguration {
    fn default() -> Self {
        Self { document_chunk_size: 150 }
    }
}