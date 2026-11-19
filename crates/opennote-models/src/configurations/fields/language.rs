use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum UserInterfaceLanguage {
    Chinese,
    #[default]
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
