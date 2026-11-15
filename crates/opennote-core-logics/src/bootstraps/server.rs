use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use opennote_data::{Databases, assets::AssetsStorage};
use opennote_models::configurations::server::ServerConfigurations;

#[derive(Clone)]
pub struct ServerBootstrap {
    pub configurations: Arc<Mutex<ServerConfigurations>>,
    pub databases: Databases,
    pub assets_storage: AssetsStorage,
}

impl ServerBootstrap {
    pub async fn new(configurations: ServerConfigurations) -> Result<Self> {
        let databases = Databases::new(&configurations.system).await?;

        let assets_storage =
            AssetsStorage::new(configurations.system.assets.assets_storage_path.clone());

        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations)),
            databases,
            assets_storage,
        })
    }
}
