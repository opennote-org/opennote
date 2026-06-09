use gpui::{App, Global};

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::configurations::{Configurations, search::SupportedSearchMethod};
use tokio::sync::MutexGuard;

use crate::{
    globals::helpers::get_configuration_filepath, key_mappings::traits::KeyMappingsUIExtension,
};

pub const SEARCH_METHODS_ENUMS: [SupportedSearchMethod; 2] = [
    SupportedSearchMethod::Keyword,
    SupportedSearchMethod::Semantic,
];

/// This is a wrapper for ApplicationBootStrap
/// We don't want to implement the UI specific trait for the object itself
pub struct GlobalApplicationBootStrap(pub ApplicationBootStrap);

impl Global for GlobalApplicationBootStrap {}

impl GlobalApplicationBootStrap {
    pub fn init(cx: &mut App, bootstrap: ApplicationBootStrap) {
        let key_bindings = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                bootstrap
                    .configurations
                    .lock()
                    .await
                    .user
                    .key_mappings
                    .clone()
                    .into_keybindings()
            })
        });

        cx.bind_keys(key_bindings);

        cx.set_global(GlobalApplicationBootStrap(bootstrap));
    }

    /// Get the configurations as a mutex guard with read-only capability
    pub fn get_configurations(&self) -> MutexGuard<'_, Configurations> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.0.configurations.lock().await })
        })
    }

    /// Return the selected search method index
    pub fn get_search_method(&self) -> usize {
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
            .save_to_file(&get_configuration_filepath())
            .unwrap();
    }
}
