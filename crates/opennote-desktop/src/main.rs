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
        assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap, states::States,
        tasks::tracker::TaskTracker, velotype::init_velotype,
    },
    libs::theme::adapt_theme_to_system,
    logs::UICustomLog,
    views::workspace::Workspace,
};

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

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        // Initialize the necessary services and resources for the app
        // 
        // TODO: Create a dedicated asset readiness panel to display the readiness, 
        // either during the startup or on user's call
        TaskTracker::init(cx);
        GlobalApplicationBootStrap::init(cx);
        AssetsCollection::init(cx)
            .context("Failed to load the assets on application start")
            .unwrap();
        States::init(cx);
        init_velotype(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                adapt_theme_to_system(cx);

                let view = cx.new(|cx| {
                    let workspace = Workspace::new(window, cx)
                        .context("Workspace initialization failed")
                        .unwrap();
                    workspace
                });

                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });

    Ok(())
}
