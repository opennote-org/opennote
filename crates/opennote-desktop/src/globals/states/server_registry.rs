use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use gpui_kit::SharedString;
use serde_encrypt::shared_key::SharedKey;
use uuid::Uuid;

use opennote_models::{
    block::Block, configurations::fields::remote_server::RemoteServerConfiguration,
    constants::LOCAL_SERVER_NAME,
};

#[derive(Debug, Clone)]
pub struct ServerRegistry(Arc<RwLock<HashMap<SharedString, ServerStates>>>);

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
                        ..Default::default()
                    },
                )
            })
            .collect();

        servers.insert(
            SharedString::new(LOCAL_SERVER_NAME),
            ServerStates::default(),
        );

        Self(Arc::new(RwLock::new(servers)))
    }

    /// It will return a hash map of the server states but without blocks cache
    pub fn get_servers_connections(&self) -> HashMap<SharedString, ServerStates> {
        self.0
            .read()
            .unwrap()
            .iter()
            .map(|(name, state)| {
                (
                    name.clone(),
                    ServerStates {
                        connection_string: state.connection_string.clone(),
                        password: state.password.clone(),
                        shared_key: state.shared_key.clone(),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    pub fn get_servers(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, HashMap<SharedString, ServerStates>> {
        self.0.read().unwrap()
    }

    fn get_servers_mut(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, HashMap<SharedString, ServerStates>> {
        self.0.write().unwrap()
    }

    /// Set the servers to not ready for cache access for updates.
    pub fn schedule_servers_cache_update(&mut self) {
        self.get_servers_mut()
            .iter_mut()
            .for_each(|(_, item)| item.start_update_cache());
    }

    /// Update a specified server.
    pub fn update_server(
        &mut self,
        server_name: &SharedString,
        closure: impl FnOnce(&mut ServerStates),
    ) {
        if let Some(server) = self.get_servers_mut().get_mut(server_name) {
            closure(server);
        }
    }

    /// If servers are still refreshing,
    /// then they will not be ready for access.
    ///
    /// TODO: we current don't enforce checking the cache readiness when accessing cache.
    /// should any issue occur, we need to enforce the readiness check.
    pub fn are_servers_ready_for_cache_access(&self) -> bool {
        self.get_servers()
            .iter()
            .all(|(_, item)| item.is_cache_ready)
    }
}

/// This stores individual server's live state info.
#[derive(Debug, Clone)]
pub struct ServerStates {
    pub connection_string: SharedString,

    pub password: SharedString,

    pub shared_key: SharedKey,

    pub blocks: HashMap<Uuid, Block>,

    /// Is this server's cache ready for access?
    pub is_cache_ready: bool,
}

impl Default for ServerStates {
    fn default() -> Self {
        Self {
            connection_string: SharedString::new(""),
            password: SharedString::new(""),
            shared_key: SharedKey::new([0u8; 32]),
            blocks: HashMap::new(),
            is_cache_ready: false,
        }
    }
}

impl ServerStates {
    pub fn start_update_cache(&mut self) {
        self.is_cache_ready = false;
    }

    pub fn finish_update_cache(&mut self) {
        self.is_cache_ready = true;
    }
}
