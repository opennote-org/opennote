pub mod globals;
pub mod key_mappings;
pub mod libs;
pub mod logs;
pub mod views;
pub mod widgets;

use anyhow::{Context, Result};
use gpui::*;
use gpui_component::*;

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::{configurations::Configurations, constants::APP_DATA_FOLDER_NAME};

use crate::{
    globals::{
        assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap,
        helpers::create_required_folders, states::States,
    },
    logs::UICustomLog,
    views::workspace::Workspace,
};

#[tokio::main]
async fn main() -> Result<()> {
    let app = Application::new();
    fast_log::init(
        fast_log::Config::new()
            .console()
            .chan_len(Some(100000))
            .level(log::LevelFilter::Trace)
            .custom(UICustomLog {}),
    )
    .unwrap();

    // TODO: Consider further restricting the input paths, thus avoiding passing
    // the config path from here
    let config_path = if let Some(config_dir) = dirs::config_dir() {
        let path = config_dir.join(APP_DATA_FOLDER_NAME);
        log::debug!(
            "Configuration directory has been set to: {}",
            path.display()
        );

        path
    } else {
        panic!("No config directory was found in this system")
    };

    create_required_folders(&config_path)?;

    // Load configurations
    let configurations =
        Configurations::load_from_file(config_path).expect("Error when loading configurations");

    let bootstrap = ApplicationBootStrap::new(configurations).await?;

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        // Initialize the necessary services and resources for the app
        States::init(cx);
        GlobalApplicationBootStrap::init(cx, bootstrap);
        AssetsCollection::init(cx)
            .context("Failed to load the assets on application start")
            .unwrap();

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                States::refresh_blocks_list(cx);

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
