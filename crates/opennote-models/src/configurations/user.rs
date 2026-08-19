//! This file defines configurations that are modifiable by individual users.
//! These are not necessarily break changes to the global uses,
//! but will directly affect the user him/herself.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::configurations::fields::{
    language::UserInterfaceLanguage, mcp_server::MCPServerConfig,
    remote_server::RemoteServerConfiguration, search::UserSearchConfiguration,
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

    /// Confugre how the opennote mcp server behaves
    pub mcp_server: MCPServerConfig,
}

impl Default for UserConfigurations {
    fn default() -> Self {
        Self {
            search: UserSearchConfiguration::default(),
            language: UserInterfaceLanguage::default(),
            remote_servers: HashMap::new(),
            mcp_server: MCPServerConfig::default(),
        }
    }
}
