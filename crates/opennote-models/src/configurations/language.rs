use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserInterfaceLanguage {
    Chinese,
    English,
}

impl Default for UserInterfaceLanguage {
    fn default() -> Self {
        UserInterfaceLanguage::English
    }
}
