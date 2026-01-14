//! This file defines configurations that are modifiable by individual users.
//! These are not necessarily break changes to the global uses,
//! but will directly affect the user him/herself.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::configurations::{
    key_mappings::KeyMappingConfiguration, search::UserSearchConfiguration,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct UserConfigurations {
    /// Configurations for search functionality
    #[serde(default)]
    pub search: UserSearchConfiguration,

    /// Configurations for key mappings
    #[serde(default)]
    pub key_mappings: KeyMappingConfiguration,
}

impl Default for UserConfigurations {
    fn default() -> Self {
        Self {
            search: UserSearchConfiguration::default(),
            key_mappings: KeyMappingConfiguration::default(),
        }
    }
}
