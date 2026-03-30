pub mod key_mappings;
pub mod search;
pub mod system;
pub mod user;

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    configurations::{system::SystemConfigurations, user::UserConfigurations},
    constants::CONFIGURATIONS_FILE_NAME,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configurations {
    /// Configurations that are relevent to how the app behaves in general
    pub system: SystemConfigurations,

    /// Configurations that are relevent to how an user uses the app
    pub user: UserConfigurations,
}

impl Default for Configurations {
    fn default() -> Self {
        Self {
            system: SystemConfigurations::default(),
            user: UserConfigurations::default(),
        }
    }
}

impl Configurations {
    /// The path to the configuration file.
    /// It will automatically add the configuration file name
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let path = if path.is_dir() {
            path.join(CONFIGURATIONS_FILE_NAME)
        } else {
            path.to_path_buf()
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let content: String = std::fs::read_to_string(&path)
            .context(format!("Failed to read config file: {}", path.display()))?;

        serde_json::from_str(&content)
            .context(format!("Failed to parse config file: {}", path.display()))
    }

    /// Reserved for future uses
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path))?;

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.system.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        Ok(())
    }
}
