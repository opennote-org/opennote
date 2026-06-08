use gpui::{App, Global};

use opennote_bootstrap::ApplicationBootStrap;
use opennote_models::configurations::search::SupportedSearchMethod;

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
        let handle = tokio::runtime::Handle::current();

        // TODO: fix the Mutex around Configurations
        let key_bindings = handle.block_on(async {
            bootstrap
                .configurations
                .lock()
                .await
                .user
                .key_mappings
                .clone()
                .into_keybindings()
        });

        cx.bind_keys(key_bindings);

        cx.set_global(GlobalApplicationBootStrap(bootstrap));
    }

    /// Return the selected search method index
    pub fn get_search_method(&self) -> usize {
        let mut selected_index = 0;
        let default_search_method = &self
            .0
            .configurations
            .blocking_lock()
            .user
            .search
            .default_search_method;

        for (index, item) in SEARCH_METHODS_ENUMS.iter().enumerate() {
            if item == default_search_method {
                selected_index = index;
            }
        }

        selected_index
    }

    pub fn set_search_method(&mut self, search_method: SupportedSearchMethod) {
        let mut configurations = self.0.configurations.blocking_lock();

        configurations.user.search.default_search_method = search_method;

        configurations.save_to_file(&get_configuration_filepath());
    }
}
