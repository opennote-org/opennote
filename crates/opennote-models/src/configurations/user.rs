//! This file defines configurations that are modifiable by individual users.
//! These are not necessarily break changes to the global uses,
//! but will directly affect the user him/herself.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::configurations::{
    fields::remote_server::RemoteServerConfiguration, language::UserInterfaceLanguage,
    search::UserSearchConfiguration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfigurations {
    /// Configurations for search functionality
    #[serde(default)]
    pub search: UserSearchConfiguration,

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
            language: UserInterfaceLanguage::default(),
            remote_servers: HashMap::new(),
        }
    }
}
