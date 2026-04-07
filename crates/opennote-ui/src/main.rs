//! Next step: Create a minimal viable gpui program with all features required by OpenNote
//! Features should be included in this MVP:
//! - [x] keyboard shortcuts
//! - [ ] ui buttons for key actions
//! - [ ] actions for calling the core APIs
//! - [x] an input panel for searching and commanding
//! - [x] multi-lingual support, a configuratble language file for all texts displaying in the program
//! - [x] a logging mechanism to display debug information to the console
//! - [x] notification support
//!
//! TODOs:
//! 1. Create a configuration handling module that will read and write configurations from a local source
//! 2. Create an API that can be called both by the server and ui

pub mod globals;
pub mod key_mappings;
pub mod views;
pub mod widgets;

use anyhow::{Context, Result};
use gpui::*;
use gpui_component::*;

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::{configurations::Configurations, constants::APP_DATA_FOLDER_NAME};

use crate::{
    globals::{assets::AssetsCollection, bootstrap::GlobalApplicationBootStrap, states::States},
    views::workspace::Workspace,
};

#[tokio::main]
async fn main() -> Result<()> {
    let app = Application::new();

    // TODO: Consider further restricting the input paths, thus avoiding passing
    // the config path from here
    let config_path = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join(APP_DATA_FOLDER_NAME)
    } else {
        panic!("No config directory was found in this system")
    };

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
                let view = cx.new(|cx| {
                    Workspace::new(window, cx)
                        .context("Workspace initialization failed")
                        .unwrap()
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
