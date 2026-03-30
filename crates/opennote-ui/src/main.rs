//! Next step: Create a minimal viable gpui program with all features required by OpenNote
//! Features should be included in this MVP:
//! 1. keyboard shortcuts
//! 2. ui buttons for key actions
//! 3. actions for calling the core APIs
//! 4. an input panel for searching and commanding
//! 5. multi-lingual support, a configuratble language file for all texts displaying in the program
//!
//! TODOs:
//! 1. Create a configuration handling module that will read and write configurations from a local source
//! 2. Create an API that can be called both by the server and ui

pub mod actions;
pub mod widgets;

use anyhow::Result;
use gpui::*;
use gpui_component::{
    sidebar::{Sidebar, SidebarMenu},
    *,
};

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::{configurations::Configurations, constants::APP_DATA_FOLDER_NAME};

pub struct Main;

impl Render for Main {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = Sidebar::new(Side::Left).child(SidebarMenu::new());

        v_flex().id("workspace-sidebar").h_full().child(sidebar)
    }
}

fn main() -> Result<()> {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            // TODO: Consider further restricting the input paths, thus avoiding passing
            // the config path from here
            let config_path = if let Some(config_dir) = dirs::config_dir() {
                config_dir.join(APP_DATA_FOLDER_NAME)
            } else {
                panic!("No config directory was found in this system")
            };

            // Load configurations
            let configurations = Configurations::load_from_file(config_path)
                .expect("Error when loading configurations");

            // Initialize the necessary services and resources for the app
            let services_and_resources = ApplicationBootStrap::new(configurations)
                .await
                .expect("Error when initializing the application");

            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Main);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });

    Ok(())
}
