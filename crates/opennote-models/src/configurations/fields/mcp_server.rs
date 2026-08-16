use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Which address should the server be listening on
    pub host: String,

    /// Which port should the server be listening on
    pub port: usize,

    /// How many threads assigned to the opennote mcp server
    pub workers: usize,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            workers: 2,
        }
    }
}

impl MCPServerConfig {
    pub fn get_mcp_server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
