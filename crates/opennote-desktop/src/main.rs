pub mod globals;
pub mod key_mappings;
pub mod libs;
pub mod logs;
pub mod views;
pub mod widgets;

use std::collections::HashMap;

use anyhow::{Context, Result};
use gpui::*;
use gpui_component::*;

use opennote_models::constants::{
    APP_DATA_FOLDER_NAME,
    env_vars::{
        DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME, STARTUP_ENVIRONMENT_VARIABLES_FOR_DESKTOP,
        set_environment_variables,
    },
};

use crate::{
    globals::{
        assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap,
        mcp_server::DesktopMCPServer, states::States, tasks::tracker::TaskTracker,
        velotype::init_velotype,
    },
    libs::theme::adapt_theme_to_system,
    logs::UICustomLog,
    views::{resource_loading::ResourceLoadingView, workspace::Workspace},
};

async fn load_startup_resources() -> Result<(GlobalApplicationBootStrap, AssetsCollection)> {
    let assets_task = tokio::task::spawn_blocking(AssetsCollection::load);
    let bootstrap = GlobalApplicationBootStrap::load().await?;
    let assets = assets_task
        .await
        .context("The asset loading task failed")??;

    Ok((bootstrap, assets))
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = Application::new().with_assets(gpui_component_assets::Assets);
    fast_log::init(
        fast_log::Config::new()
            .console()
            .chan_len(Some(100000))
            .level(log::LevelFilter::Debug)
            .custom(UICustomLog {}),
    )
    .unwrap();

    set_environment_variables(
        &STARTUP_ENVIRONMENT_VARIABLES_FOR_DESKTOP,
        HashMap::from([(
            DEFAULT_SQLITE_DATA_FOLDER_NAME_ENV_VAR_NAME,
            APP_DATA_FOLDER_NAME,
        )]),
    )?;

    let tokio_handle = tokio::runtime::Handle::current();
    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        TaskTracker::init(cx);

        let loading_window =
            ResourceLoadingView::open(cx).expect("Failed to open the resource loading window");

        cx.spawn(async move |cx| {
            let resources = tokio_handle
                .spawn(load_startup_resources())
                .await
                .context("The startup resource task failed")
                .and_then(|resources| resources);

            let (bootstrap, assets) = match resources {
                Ok(resources) => resources,
                Err(error) => {
                    log::error!("Failed to initialize OpenNote: {error:#}");
                    let message = format!("{error:#}");
                    let _ = loading_window.update(cx, |view, _window, cx| {
                        view.set_error(message, cx);
                    });
                    return;
                }
            };

            let _ = loading_window.update(cx, move |loading_view, loading_window, cx| {
                bootstrap.install(cx);
                cx.set_global(assets);
                States::init(cx);

                match init_velotype(cx) {
                    Ok(_) => {}
                    Err(error) => loading_view.set_error(error.to_string(), cx),
                };

                match DesktopMCPServer::init(cx) {
                    Ok(_) => {}
                    Err(error) => loading_view.set_error(error.to_string(), cx),
                };

                let workspace_window = cx.open_window(WindowOptions::default(), |window, cx| {
                    adapt_theme_to_system(cx);

                    let view = cx.new(|cx| {
                        Workspace::new(window, cx)
                            .context("Workspace initialization failed")
                            .unwrap()
                    });

                    // This first level on the window should be a Root.
                    cx.new(|cx| Root::new(view, window, cx))
                });

                match workspace_window {
                    Ok(_) => loading_window.remove_window(),
                    Err(error) => {
                        log::error!("Failed to open the Workspace window: {error:#}");
                        loading_view.set_error(
                            format!("Failed to open the Workspace window: {error:#}"),
                            cx,
                        );
                    }
                }
            });
        })
        .detach();
    });

    Ok(())
}
