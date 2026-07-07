pub mod key_mappings;
pub mod language;
pub mod remote_server;
pub mod search;
pub mod system;
pub mod user;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    configurations::{system::SystemConfigurations, user::UserConfigurations},
    constants::CONFIGURATIONS_FILE_NAME,
    traits::LoadFromAndSaveToFile,
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
    pub fn validate(&self) -> Result<()> {
        if self.system.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        Ok(())
    }
}

impl LoadFromAndSaveToFile for Configurations {
    fn get_configuration_filename() -> &'static str {
        CONFIGURATIONS_FILE_NAME
    }
}
