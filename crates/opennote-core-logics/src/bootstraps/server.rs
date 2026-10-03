use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use opennote_data::Databases;
use opennote_models::configurations::server::ServerConfigurations;

#[derive(Clone)]
pub struct ServerBootstrap {
    pub configurations: Arc<Mutex<ServerConfigurations>>,
    pub databases: Databases,
}

impl ServerBootstrap {
    pub async fn new(configurations: ServerConfigurations) -> Result<Self> {
        let databases = Databases::new(&configurations.system).await?;

        Ok(Self {
            configurations: Arc::new(Mutex::new(configurations)),
            databases,
        })
    }
}
