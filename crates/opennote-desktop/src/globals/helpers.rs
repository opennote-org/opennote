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

/// Run async codes in sync functions
pub fn run_async_code<F, R>(closure: F) -> R
where
    F: Future<Output = R>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(closure))
}
