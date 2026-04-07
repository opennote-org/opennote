use anyhow::Result;

use crate::globals::{
    assets::{AssetsCollection, LanguageProfile},
    bootstrap::GlobalApplicationBootStrap,
};

pub fn get_language_profile(
    bootstrap: &GlobalApplicationBootStrap,
    assets_collection: &AssetsCollection,
) -> Result<LanguageProfile> {
    let language = bootstrap.0.configurations.user.language.to_string();

    Ok(assets_collection
        .language_profiles
        .get(&language)
        .unwrap()
        .to_owned())
}
