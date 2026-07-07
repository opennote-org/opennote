//! This file defines configurations that are modifiable by individual users.
//! These are not necessarily break changes to the global uses,
//! but will directly affect the user him/herself.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::configurations::{
    key_mappings::KeyMappings, language::UserInterfaceLanguage,
    remote_server::RemoteServerConfiguration, search::UserSearchConfiguration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfigurations {
    /// Configurations for search functionality
    #[serde(default)]
    pub search: UserSearchConfiguration,

    /// Configurations for key mappings
    #[serde(default)]
    pub key_mappings: KeyMappings,

    /// The language used in the user interface
    pub language: UserInterfaceLanguage,

    /// The remote servers to connect to
    #[serde(default)]
    pub remote_servers: HashMap<String, RemoteServerConfiguration>,
}

impl Default for UserConfigurations {
    fn default() -> Self {
        Self {
            search: UserSearchConfiguration::default(),
            key_mappings: KeyMappings::default(),
            language: UserInterfaceLanguage::default(),
            remote_servers: HashMap::new(),
        }
    }
}
