use std::collections::HashMap;

use anyhow::Result;
use gpui_kit::App;

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

/// Run async codes in the background without freezing the foreground UI
///
/// Examples:
/// opennote/crates/opennote-desktop/src/globals/actions/mod.rs at line 287
/// opennote/crates/opennote-desktop/src/views/workspace/actions.rs at line 309
pub async fn run_async_background<F, V>(
    executor: &gpui_kit::BackgroundExecutor,
    tokio_handle: tokio::runtime::Handle,
    closure: F,
) -> V
where
    F: Future<Output = V> + Send + 'static,
    F::Output: Send + 'static,
{
    executor
        .spawn(async move { tokio_handle.spawn(closure).await.unwrap() })
        .await
}
