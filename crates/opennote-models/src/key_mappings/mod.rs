use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    constants::KEY_MAPPINGS_FILE_NAME,
    traits::{LoadFromAndSaveToFile, MigrateConfigurationFileStructure},
};

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct KeyMappingConfigurations {
    pub conventional: KeyMappings,
    pub vim: KeyMappings,
}

impl Default for KeyMappingConfigurations {
    fn default() -> Self {
        Self {
            conventional: KeyMappings::get_default_conventional_key_mappings(),
            vim: KeyMappings::get_default_vim_key_mappings(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(transparent)]
pub struct KeyMappings(pub Vec<KeyMapping>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema, Eq)]
pub struct KeyMapping {
    /// Keys to trigger this binding
    /// For pressing all together, just put each key in this form:
    /// ["cmd", "-", "b"]
    /// For sequential key presses, like Vim, just put each key in this form:
    /// ["g", "", "g"]
    pub sequence: Vec<String>,
    /// The action that this key binding associates to.
    /// Should be in CamelCase
    pub action: String,
    /// In which context, should this key binding is available
    pub context: String,
}

impl LoadFromAndSaveToFile for KeyMappingConfigurations {
    fn get_configuration_filename() -> &'static str {
        KEY_MAPPINGS_FILE_NAME
    }
}

impl MigrateConfigurationFileStructure for KeyMappingConfigurations {}

impl KeyMappings {
    fn get_default_vim_key_mappings() -> Self {
        Self(vec![
            // Workspace
            KeyMapping {
                sequence: vec![" ".to_string(), "".to_string(), "b".to_string()],
                action: format!("ToggleSidebar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["/".to_string()],
                action: format!("ToggleSearchBar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec![":".to_string()],
                action: format!("ToggleCommandBar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec![" ".to_string(), "".to_string(), ";".to_string()],
                action: format!("ToggleSettingsPanel"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec![" ".to_string(), "".to_string(), "n".to_string()],
                action: format!("CreateOneBlock"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["shift".to_string(), "-".to_string(), "k".to_string()],
                action: format!("NextTab"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["shift".to_string(), "-".to_string(), "j".to_string()],
                action: format!("PreviousTab"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec![" ".to_string(), "".to_string(), "x".to_string()],
                action: format!("CloseActiveTab"),
                context: "workspace".to_string(),
            },
            // Workspace Sidebar
            KeyMapping {
                sequence: vec!["d".to_string(), "".to_string(), "d".to_string()],
                action: format!("DeleteBlocks"),
                context: "workspace_sidebar".to_string(),
            },
            // Editor
            KeyMapping {
                sequence: vec!["ctrl".to_string(), "-".to_string(), "s".to_string()],
                action: format!("SaveDocument"),
                context: "editor".to_string(),
            },
            // General
            KeyMapping {
                sequence: vec!["k".to_string()],
                action: format!("MoveUp"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["j".to_string()],
                action: format!("MoveDown"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["h".to_string()],
                action: format!("MoveLeft"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["l".to_string()],
                action: format!("MoveRight"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["space".to_string()],
                action: format!("Open"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["x".to_string()],
                action: format!("Delete"),
                context: "general".to_string(),
            },
        ])
    }

    fn get_default_conventional_key_mappings() -> Self {
        Self(vec![
            // Workspace
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), "b".to_string()],
                action: format!("ToggleSidebar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec![
                    "cmd".to_string(),
                    "-".to_string(),
                    "shift".to_string(),
                    "-".to_string(),
                    "f".to_string(),
                ],
                action: format!("ToggleSearchBar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), ":".to_string()],
                action: format!("ToggleCommandBar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), ";".to_string()],
                action: format!("ToggleSettingsPanel"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), "n".to_string()],
                action: format!("CreateOneBlock"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["ctrl".to_string(), "-".to_string(), "tab".to_string()],
                action: format!("NextTab"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec![
                    "ctrl".to_string(),
                    "-".to_string(),
                    "shift".to_string(),
                    "-".to_string(),
                    "tab".to_string(),
                ],
                action: format!("PreviousTab"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["ctrl".to_string(), "-".to_string(), "w".to_string()],
                action: format!("CloseActiveTab"),
                context: "workspace".to_string(),
            },
            // Workspace Sidebar
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), "d".to_string()],
                action: format!("DeleteBlocks"),
                context: "workspace_sidebar".to_string(),
            },
            // Editor
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), "s".to_string()],
                action: format!("SaveDocument"),
                context: "editor".to_string(),
            },
            // General
            KeyMapping {
                sequence: vec!["up".to_string()],
                action: format!("MoveUp"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["down".to_string()],
                action: format!("MoveDown"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["left".to_string()],
                action: format!("MoveLeft"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["right".to_string()],
                action: format!("MoveRight"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["enter".to_string()],
                action: format!("Open"),
                context: "general".to_string(),
            },
            KeyMapping {
                sequence: vec!["backspace".to_string()],
                action: format!("Delete"),
                context: "general".to_string(),
            },
        ])
    }
}
