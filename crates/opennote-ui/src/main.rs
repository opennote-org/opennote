//! Next step: Create a minimal viable gpui program with all features required by OpenNote
//! Features should be included in this MVP:
//! - [x] keyboard shortcuts
//! - [ ] ui buttons for key actions
//! - [ ] actions for calling the core APIs
//! - [ ] an input panel for searching and commanding
//! - [ ] multi-lingual support, a configuratble language file for all texts displaying in the program
//!
//! TODOs:
//! 1. Create a configuration handling module that will read and write configurations from a local source
//! 2. Create an API that can be called both by the server and ui

pub mod actions;
pub mod globals;
pub mod key_mappings;
pub mod screens;
pub mod widgets;
pub mod library;

use anyhow::Result;
use gpui::*;
use gpui_component::*;

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::{configurations::Configurations, constants::APP_DATA_FOLDER_NAME};

use crate::{
    globals::UIApplicationBootStrap, key_mappings::traits::KeyMappingsUIExtension,
    screens::workspace::Workspace,
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

    // Initialize the necessary services and resources for the app
    let services_and_resources = UIApplicationBootStrap(
        ApplicationBootStrap::new(configurations)
            .await
            .expect("Error when initializing the application"),
    );

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.set_global(services_and_resources);

        let services_and_resources: &UIApplicationBootStrap = cx.global();

        cx.bind_keys(
            services_and_resources
                .0
                .configurations
                .user
                .key_mappings
                .clone()
                .into_keybindings(),
        );

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| Workspace::new(cx));
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });

    Ok(())
}
