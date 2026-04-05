use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserInterfaceLanguage {
    Chinese,
    English,
}

impl Display for UserInterfaceLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserInterfaceLanguage::Chinese => f.write_str("chinese"),
            UserInterfaceLanguage::English => f.write_str("english"),
        }
    }
}

impl Default for UserInterfaceLanguage {
    fn default() -> Self {
        UserInterfaceLanguage::English
    }
}
