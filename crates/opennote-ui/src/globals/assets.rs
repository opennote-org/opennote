use std::collections::HashMap;

use anyhow::Result;
use gpui::Global;
use rust_embed::Embed;
use serde::{Deserialize, Serialize};

#[derive(Embed)]
#[folder = "../../assets"]
#[include = "*.json"]
pub struct Assets;

#[derive(Debug, Clone)]
pub struct AssetsCollection {
    pub language_profiles: HashMap<String, LanguageProfile>,
}

impl AssetsCollection {
    pub fn load() -> Result<Self> {
        let mut language_profiles = HashMap::new();

        for file in Assets::iter() {
            if file.contains("languages/") {
                if let Some(embedded_file) = Assets::get(&file) {
                    let language_profile: LanguageProfile =
                        serde_json::from_slice(&embedded_file.data.as_ref())?;
                    
                    let language = file
                        .trim_start_matches("languages/")
                        .trim_end_matches(".json")
                        .to_string();

                    language_profiles.insert(
                        language,
                        language_profile,
                    );
                }
            }
        }

        Ok(Self { language_profiles })
    }
}

impl Global for AssetsCollection {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageProfile {
    pub search_bar_placeholder: String,
}
