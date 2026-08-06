use std::collections::HashMap;

use anyhow::Result;
use gpui::App;

use crate::globals::{assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap};

pub fn get_language_profile(cx: &App) -> Result<HashMap<String, String>> {
    let bootstrap: &GlobalApplicationBootStrap = cx.global();
    let assets_collection: &AssetsCollection = cx.global();
    let configurations = bootstrap.get_configurations();

    let language = configurations.user.language.to_string();

    Ok(assets_collection
        .language_profiles
        .get(&language)
        .unwrap()
        .to_owned())
}
