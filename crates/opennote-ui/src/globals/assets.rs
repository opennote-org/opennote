use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::globals::traits::Load;

#[derive(Debug, Clone)]
pub struct AssetsCollection {
    pub language_profiles: HashMap<String, LanguageProfile>
}

impl AssetsCollection {
    pub fn load() -> Result<Self> {
        
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageProfile {
    pub search_bar_placeholder: String
}

impl Load for LanguageProfile {
    fn load() -> Result<Self> {
        Ok(Self { search_bar_placeholder: "".to_string() })
    }
}