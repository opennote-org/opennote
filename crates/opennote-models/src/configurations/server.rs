use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_encrypt::shared_key::SharedKey;

use crate::{
    configurations::system::SystemConfigurations, constants::CONFIGURATIONS_FILE_NAME,
    traits::LoadFromAndSaveToFile,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfigurations {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub system: SystemConfigurations,

    /// The shared key is set for this server
    pub shared_key: SharedKey,
}

impl Default for ServerConfigurations {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 4,
            system: SystemConfigurations::default(),
            shared_key: SharedKey::new_const([0u8; 32]),
        }
    }
}

impl ServerConfigurations {
    pub fn validate(&self) -> Result<()> {
        if self.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        Ok(())
    }
}

impl LoadFromAndSaveToFile for ServerConfigurations {
    fn get_configuration_filename() -> &'static str {
        CONFIGURATIONS_FILE_NAME
    }
}
