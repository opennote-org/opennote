use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Whether the MCP is enabled
    pub enabled: bool,

    /// Which address should the server be listening on
    pub host: String,

    /// Which port should the server be listening on
    pub port: usize,

    /// How many threads assigned to the opennote mcp server
    pub workers: usize,

    /// Which server to allow AI agents to operate on.
    /// Empty means all, including the local one.
    /// `local` means the local server.
    #[serde(default)]
    pub scopes: Vec<String>,

    /// Whether allowing AI agents to perform read operations via the mcp server
    #[serde(default)]
    pub allow_read: bool,

    /// Whether allowing AI agents to perform
    /// create, delete and update operations via the mcp server
    #[serde(default)]
    pub allow_write: bool,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "localhost".to_string(),
            port: 8080,
            workers: 2,
            scopes: Vec::new(),
            allow_read: true,
            allow_write: false,
        }
    }
}

impl MCPServerConfig {
    pub fn get_mcp_server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn are_all_scopes_allowed(&self) -> bool {
        self.scopes.is_empty()
    }

    pub fn validate_scopes(&self, server_name: String) -> bool {
        self.scopes.contains(&server_name) || self.are_all_scopes_allowed()
    }

    /// This will throw an error if the required access is not granted
    pub fn validate_access(&self, require_write: bool, require_read: bool) -> Result<()> {
        if !self.allow_write && require_write {
            return Err(anyhow!(
                "MCP server was not configured to write by the owner"
            ));
        }

        if !self.allow_read && require_read {
            return Err(anyhow!(
                "MCP server was not configured to read by the owner"
            ));
        }

        Ok(())
    }
}
