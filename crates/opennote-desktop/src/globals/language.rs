use std::collections::HashMap;

use anyhow::Result;
use gpui_kit::{App, SharedString};

use crate::globals::{assets::AssetsCollection, bootstrap::helpers::get_bootstrap};

#[derive(Debug, Clone)]
pub struct LanguageProfile(HashMap<String, String>);

impl LanguageProfile {
    pub fn new(profile: HashMap<String, String>) -> Self {
        Self(profile)
    }

    pub fn get_description(&self, key: &str) -> Option<SharedString> {
        if let Some(description) = self.0.get(key) {
            return Some(description.into());
        }

        None
    }

    /// Get a language profile that suits to the user's language setting
    pub fn get_language_profile(cx: &App) -> Result<LanguageProfile> {
        let bootstrap = get_bootstrap(cx);
        let assets_collection: &AssetsCollection = cx.global();
        let configurations = bootstrap.get_configurations();

        let language = configurations.user.language.to_string();

        Ok(LanguageProfile::new(
            assets_collection
                .language_profiles
                .get(&language)
                .unwrap()
                .to_owned(),
        ))
    }
}
