use serde::{Deserialize, Serialize};

use crate::{
    configurations::{system::SystemConfigurations, user::UserConfigurations},
    constants::CONFIGURATIONS_FILE_NAME,
    traits::{LoadFromAndSaveToFile, MigrateConfigurationFileStructure},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfigurations {
    /// Configurations that are relevent to how the app behaves in general
    pub system: SystemConfigurations,

    /// Configurations that are relevent to how an user uses the app
    pub user: UserConfigurations,
}

impl Default for DesktopConfigurations {
    fn default() -> Self {
        Self {
            system: SystemConfigurations::default(),
            user: UserConfigurations::default(),
        }
    }
}

impl LoadFromAndSaveToFile for DesktopConfigurations {
    fn get_configuration_filename() -> &'static str {
        CONFIGURATIONS_FILE_NAME
    }
}

impl MigrateConfigurationFileStructure for DesktopConfigurations {}
