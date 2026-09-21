pub mod globals;
pub mod key_mappings;
pub mod libs;
pub mod views;
pub mod widgets;
pub mod window;

use std::collections::HashMap;

use anyhow::{Context, Result};
use gpui_kit::{component::*, *};

use opennote_core_logics::logging::initialize_logger;
use opennote_models::constants::{
    APP_DATA_FOLDER_NAME, LOG_WINDOW_CAPACITY,
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
    views::{
        log::{LogWindow, writer::WindowLogWriter},
        resource_loading::ResourceLoadingView,
        workspace::Workspace,
    },
    window::{create_main_window_option, format_window_title},
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
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

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
        gpui_kit::init(cx);
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
                    let message = format!("{error:#}");
                    let _ = loading_window.update(cx, |view, _window, cx| {
                        view.set_error(message, cx);
                    });
                    return;
                }
            };

            // Initialize a logger in the background to stream logs into the log window
            let (sender, receiver) = std::sync::mpsc::sync_channel(LOG_WINDOW_CAPACITY);

            initialize_logger(
                &bootstrap.get_configurations().system.logging.level,
                Some(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(true)
                        .with_writer(WindowLogWriter::new(sender)),
                ),
            );

            let _ = loading_window.update(cx, move |loading_view, loading_window, cx| {
                LogWindow::install(receiver, cx);
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

                let workspace_window = cx.open_window(
                    create_main_window_option(format_window_title(None, None, None)),
                    |window, cx| {
                        let view = cx.new(|cx| {
                            Workspace::new(window, cx)
                                .context("Workspace initialization failed")
                                .unwrap()
                        });

                        // This first level on the window should be a Root.
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                );

                match workspace_window {
                    Ok(_) => loading_window.remove_window(),
                    Err(error) => {
                        tracing::error!("Failed to open the Workspace window: {error:#}");
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
