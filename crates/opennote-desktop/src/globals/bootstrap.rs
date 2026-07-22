use anyhow::Context;
use gpui::{App, Global};

use tokio::sync::MutexGuard;

use opennote_bootstrap::DesktopBootstrap;
use opennote_core_logics::configurations::{
    ApplicationType, create_required_folders, get_configuration_folder_path,
};
use opennote_data::search::SearchScope;
use opennote_models::{
    configurations::{desktop::DesktopConfigurations, search::SupportedSearchMethod},
    key_mappings::KeyMappingConfigurations,
    traits::{LoadFromAndSaveToFile, MigrateConfigurationFileStructure},
};

use crate::{globals::helpers::run_async_code, key_mappings::traits::KeyMappingsUIExtension};

pub const SEARCH_METHODS_ENUMS: [SupportedSearchMethod; 2] = [
    SupportedSearchMethod::Keyword,
    SupportedSearchMethod::Semantic,
];

pub const SEARCH_SCOPES_ENUMS: [SearchScope; 3] = [
    SearchScope::Document,
    SearchScope::Collection,
    SearchScope::Userspace,
];

/// This is a wrapper for DesktopBootstrap
/// We don't want to implement the UI specific trait for the object itself
pub struct GlobalApplicationBootStrap(pub DesktopBootstrap);

impl Global for GlobalApplicationBootStrap {}

impl GlobalApplicationBootStrap {
    pub fn init(cx: &mut App) {
        let config_path: std::path::PathBuf =
            get_configuration_folder_path(ApplicationType::Desktop);

        create_required_folders(&config_path)
            .context("Failed to create required folders")
            .unwrap();

        // Load configurations
        let configurations = DesktopConfigurations::load_from_file(&config_path)
            .context("Failed to load configurations on application start")
            .unwrap()
            .migrate(&config_path)
            .unwrap();

        let key_mappings = KeyMappingConfigurations::load_from_file(&config_path)
            .context("Failed to load key mappings on application start")
            .unwrap()
            .migrate(&config_path)
            .unwrap();

        let bootstrap = run_async_code(async {
            DesktopBootstrap::new(&configurations, &key_mappings)
                .await
                .context("Failed to bootstrap the application")
                .unwrap()
        });

        let key_bindings = run_async_code(async {
            // TODO: Add vim support
            let conventional = bootstrap
                .key_mappings
                .lock()
                .await
                .conventional
                .clone()
                .into_keybindings();

            conventional
        });
        cx.bind_keys(key_bindings);

        cx.set_global(GlobalApplicationBootStrap(bootstrap));
    }

    /// Get the configurations as a mutex guard with read-only capability
    pub fn get_configurations(&self) -> MutexGuard<'_, DesktopConfigurations> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.0.configurations.lock().await })
        })
    }

    /// Return the selected search method index
    pub fn get_search_method_index(&self) -> usize {
        let mut selected_index = 0;
        let default_search_method = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.0
                    .configurations
                    .lock()
                    .await
                    .user
                    .search
                    .default_search_method
            })
        });

        for (index, item) in SEARCH_METHODS_ENUMS.iter().enumerate() {
            if *item == default_search_method {
                selected_index = index;
            }
        }

        selected_index
    }

    pub fn set_search_method(&mut self, search_method: SupportedSearchMethod) {
        let mut configurations = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.0.configurations.lock().await })
        });

        configurations.user.search.default_search_method = search_method;

        configurations
            .save_to_file(&get_configuration_folder_path(ApplicationType::Desktop))
            .unwrap();
    }
}
