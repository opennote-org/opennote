use std::collections::HashMap;

use gpui::SharedString;
use serde_encrypt::shared_key::SharedKey;
use uuid::Uuid;

use opennote_models::{
    block::Block, configurations::fields::remote_server::RemoteServerConfiguration,
    constants::LOCAL_SERVER_NAME,
};

#[derive(Debug, Clone)]
pub struct ServerRegistry(HashMap<SharedString, ServerStates>);

impl ServerRegistry {
    /// This will also include the local workspace as a server too.
    pub fn build_servers(servers: HashMap<String, RemoteServerConfiguration>) -> Self {
        let mut servers: HashMap<SharedString, ServerStates> = servers
            .into_iter()
            .map(|(server_name, config)| {
                (
                    server_name.into(),
                    ServerStates {
                        connection_string: config.connection_string.into(),
                        password: config.password.into(),
                        shared_key: config.shared_key.clone(),
                        blocks: HashMap::new(),
                    },
                )
            })
            .collect();

        servers.insert(
            SharedString::new(LOCAL_SERVER_NAME),
            ServerStates {
                connection_string: SharedString::new(""),
                password: SharedString::new(""),
                shared_key: SharedKey::new([0u8; 32]),
                blocks: HashMap::new(),
            },
        );

        Self(servers)
    }

    pub fn get_servers(&self) -> &HashMap<SharedString, ServerStates> {
        &self.0
    }

    pub fn get_servers_mut(&mut self) -> &mut HashMap<SharedString, ServerStates> {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub struct ServerStates {
    pub connection_string: SharedString,
    pub password: SharedString,
    pub shared_key: SharedKey,
    pub blocks: HashMap<Uuid, Block>,
}
