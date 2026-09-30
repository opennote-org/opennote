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

/// Run async codes in the background without freezing the foreground UI.
/// This is for running async functions that require a tokio runtime
/// and also require direct value returning.
///
/// Examples:
///
/// opennote/crates/opennote-desktop/src/globals/actions/mod.rs at line 287
/// ```
/// let vectorized_payloads =
///     run_async_background(executor, tokio_handle.clone(), async move {
///         vectorize(&embedders, &embedders_config, payloads).await
///     })
///     .await;
/// ```
///
/// opennote/crates/opennote-desktop/src/views/workspace/actions.rs at line 309
/// ```
/// let result = run_async_background(
///     executor, tokio_handle.clone(), async move {
///         build_block(
///             parent_block_id,
///             raw_file_name,
///             &embedders,
///             Some(content),
///             Some(document_chunk_size),
///         ).await
///     }
/// ).await;
/// ```
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

/// Run async codes in the background without freezing the foreground UI.
/// This is for running async functions that require a tokio runtime
/// and does not require direct value returning.
///
/// Examples:
///
/// opennote/crates/opennote-desktop/src/startup.rs at `load_resources`
/// ```
/// run_async_background_detached(cx.background_executor(), tokio_handle, async move {
///     message_sender
///         .send("Loading app bootstraps...")
///         .await
///         .unwrap();
///
///     let bootstrap = match GlobalApplicationBootStrap::load().await {
///         Ok(result) => result,
///         Err(error) => {
///             error_sender.send(error).unwrap();
///             return;
///         }
///     };
///
///     message_sender.send("Loading assets...").await.unwrap();
///     let assets = match AssetsCollection::load() {
///         Ok(result) => result,
///         Err(error) => {
///             error_sender.send(error).unwrap();
///             return;
///         }
///     };
///
///     let _ = resource_sender.send((bootstrap, assets));
/// })
/// .await;
/// ```
pub async fn run_async_background_detached<F, V>(
    executor: &gpui_kit::BackgroundExecutor,
    tokio_handle: tokio::runtime::Handle,
    closure: F,
) where
    F: Future<Output = V> + Send + 'static,
    F::Output: Send + 'static,
{
    executor
        .spawn(async move { tokio_handle.spawn(closure).await.unwrap() })
        .detach();
}
