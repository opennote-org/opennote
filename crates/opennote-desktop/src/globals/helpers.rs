use std::collections::HashMap;

use anyhow::Result;

use crate::globals::{assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap};

pub fn get_language_profile(
    bootstrap: &GlobalApplicationBootStrap,
    assets_collection: &AssetsCollection,
) -> Result<HashMap<String, String>> {
    let configurations = bootstrap.get_configurations();

    let language = configurations.user.language.to_string();

    Ok(assets_collection
        .language_profiles
        .get(&language)
        .unwrap()
        .to_owned())
}
