use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
pub struct KeyMappings(pub Vec<KeyMapping>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
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

impl Default for KeyMappings {
    fn default() -> Self {
        Self(vec![
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
                    "p".to_string(),
                ],
                action: format!("ToggleSearchBar"),
                context: "workspace".to_string(),
            },
            KeyMapping {
                sequence: vec!["cmd".to_string(), "-".to_string(), "n".to_string()],
                action: format!("CreateOneBlock"),
                context: "workspace".to_string(),
            },
        ])
    }
}
