pub mod helpers;

use anyhow::{Context, Result};
use async_trait::async_trait;
use gpui_kit::{Action, App, Global};

use tokio::sync::{MutexGuard, mpsc::Sender};

use opennote_core_logics::{
    bootstraps::desktop::DesktopBootstrap,
    configurations::{ApplicationType, create_required_folders, get_configuration_folder_path},
    helpers::run_async_code,
};
use opennote_data::search::SearchScope;
use opennote_models::{
    configurations::{desktop::DesktopConfigurations, fields::search::SupportedSearchMethod},
    key_mappings::KeyMappingConfigurations,
    metadata::Metadata,
    traits::{LoadFromAndSaveToFile, MigrateConfigurationFileStructure},
};

use crate::{
    key_mappings::{mappings::Reindex, traits::KeyMappingsUIExtension},
    startup::traits::InitializeAsResourceOnAppStart,
};

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

#[async_trait]
impl InitializeAsResourceOnAppStart for GlobalApplicationBootStrap {
    async fn initialize_as_resource(
        message_sender: &Sender<&'static str>,
        dispatch_actions: &Sender<Box<dyn Action>>,
    ) -> Result<Self> {
        message_sender
            .send("Loading app bootstraps...")
            .await
            .unwrap();
        let bootstrap = GlobalApplicationBootStrap::load().await?;

        // Now, the startup will check the local indexes.
        // The server indexes will be checked after the program started.
        message_sender.send("Check local indexes...").await?;
        let handling = bootstrap.0.analyze_changes_handling().await?;

        if handling.reindex_vector_database || handling.reset_vector_database {
            dispatch_actions.send(Box::new(Reindex)).await.unwrap();
        }

        Ok(bootstrap)
    }
}

impl GlobalApplicationBootStrap {
    pub async fn load() -> anyhow::Result<Self> {
        let (configurations, key_mappings) = tokio::task::spawn_blocking(move || {
            let config_path = get_configuration_folder_path(ApplicationType::Desktop);

            create_required_folders(&config_path).context("Failed to create required folders")?;

            let configurations = DesktopConfigurations::load_from_file(&config_path)
                .context("Failed to load configurations on application start")?
                .migrate(&config_path)
                .context("Failed to migrate configurations on application start")?;

            let key_mappings = KeyMappingConfigurations::load_from_file(&config_path)
                .context("Failed to load key mappings on application start")?
                .migrate(&config_path)
                .context("Failed to migrate key mappings on application start")?;

            Metadata::load_from_file(&config_path)
                .context("Failed to load metadata on application start")?;

            Ok::<_, anyhow::Error>((configurations, key_mappings))
        })
        .await
        .context("The resource loading task failed")??;

        // TODO: Need to send reindex request to the connected servers as well
        let bootstrap = DesktopBootstrap::new(configurations.clone(), key_mappings)
            .await
            .context("Failed to bootstrap the application")?;

        Ok(Self(bootstrap))
    }

    pub fn install(self, cx: &mut App) {
        let key_bindings = run_async_code(async {
            // TODO: Add vim support
            self.0
                .key_mappings
                .lock()
                .await
                .conventional
                .clone()
                .into_keybindings()
        });
        cx.bind_keys(key_bindings);
        cx.set_global(self);
    }

    /// Get the configurations as a mutex guard with read-only capability
    pub fn get_configurations(&self) -> MutexGuard<'_, DesktopConfigurations> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.0.configurations.lock().await })
        })
    }
}
