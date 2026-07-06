use std::collections::HashMap;

use gpui::{App, Global, SharedString, WeakEntity};
use serde_encrypt::shared_key::SharedKey;
use uuid::Uuid;

use opennote_data::{Databases, database::enums::BlockQuery, search::SearchScope};
use opennote_models::{
    block::Block, configurations::remote_server::RemoteServerConfiguration,
    constants::LOCAL_SERVER_NAME,
};

use crate::{
    globals::{
        actions::route_helpers::route_read_blocks,
        bootstrap::{GlobalApplicationBootStrap, SEARCH_SCOPES_ENUMS},
        helpers::run_async_code,
    },
    widgets::pane::pane::Pane,
};

#[derive(Debug, Clone)]
pub struct ServerStates {
    pub connection_string: SharedString,
    pub password: SharedString,
    pub shared_key: SharedKey,
    pub blocks: HashMap<Uuid, Block>,
}

/// It manages general global states
pub struct States {
    /// States of the remote servers
    servers: HashMap<SharedString, ServerStates>,

    /// The active server
    pub active_server: SharedString,

    /// The pane that is active.
    /// It is optional because we can't create a pane when new.
    pub active_pane: Option<WeakEntity<Pane>>,

    pub search_scope: SearchScope,
}

impl Global for States {}

impl States {
    pub fn new(servers: HashMap<String, RemoteServerConfiguration>) -> Self {
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

        Self {
            active_server: SharedString::new(LOCAL_SERVER_NAME),
            servers,
            active_pane: None,
            search_scope: SearchScope::Document,
        }
    }

    pub fn init(cx: &mut App) {
        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let remote_server_configs = run_async_code(async {
            bootstrap
                .0
                .configurations
                .lock()
                .await
                .user
                .remote_servers
                .clone()
        });

        let states = States::new(remote_server_configs);
        states.refresh_blocks_list(cx);

        cx.set_global(states);
    }

    /// Overwrite the existing blocks in the states with the new blocks
    pub fn hard_update_blocks(&mut self, server_name: &SharedString, blocks: Vec<Block>) {
        if let Some(server) = self.servers.get_mut(server_name) {
            server.blocks = HashMap::from_iter(blocks.into_iter().map(|item| (item.id, item)));
        }
    }

    /// It will refresh blocks across all servers
    pub fn refresh_blocks_list(&self, cx: &mut App) {
        log::debug!("Refreshing blocks...");

        let servers = self.get_servers().to_owned();

        cx.spawn(async move |cx| {
            let databases = cx
                .read_global::<GlobalApplicationBootStrap, Databases>(|this, _cx| {
                    this.0.databases.clone()
                })
                .unwrap();

            let handles: Vec<_> = servers
                .into_iter()
                .map(|(name, server)| {
                    let databases = databases.clone();

                    async move {
                        match route_read_blocks(&name, &server, &databases, &BlockQuery::All).await
                        {
                            Ok(results) => (name, Ok(results)),
                            Err(error) => {
                                log::error!("{}", error);
                                (name, Err(error))
                            }
                        }
                    }
                })
                .collect();

            let results = futures::future::join_all(handles).await;

            for (name, result) in results {
                if let Ok(blocks) = result {
                    match cx.update_global::<States, ()>(|this, _cx| {
                        this.hard_update_blocks(&name, blocks);
                    }) {
                        Ok(_) => {}
                        Err(error) => log::error!("{}", error),
                    }
                }
            }
        })
        .detach();
    }

    pub fn get_block(&self, block_id: &Uuid) -> Option<Block> {
        for (_name, server) in self.servers.iter() {
            match server.blocks.get(&block_id) {
                Some(block) => return Some(block.clone()),
                _ => {}
            }
        }

        None
    }

    /// Get all blocks ids from all servers
    pub fn get_all_blocks_ids(&self) -> Vec<Uuid> {
        let mut block_ids = Vec::new();

        for (_name, server) in self.servers.iter() {
            block_ids.extend(
                server
                    .blocks
                    .iter()
                    .map(|(block_id, _)| *block_id)
                    .collect::<Vec<Uuid>>(),
            );
        }

        block_ids
    }

    /// Return an empty vec if nothing is found
    pub fn get_all_blocks_by_server(&self, server_name: &SharedString) -> Vec<Block> {
        for (name, server) in self.servers.iter() {
            if server_name == name {
                return server
                    .blocks
                    .iter()
                    .map(|(_id, item)| item.clone())
                    .collect();
            }
        }

        Vec::new()
    }

    pub fn find_block_children_ids(&self, block_id: Uuid) -> Vec<Uuid> {
        let mut blocks = Vec::new();

        for (_name, server) in self.servers.iter() {
            blocks.extend(
                server
                    .blocks
                    .iter()
                    .filter_map(|(_, block)| {
                        if block.parent_id == Some(block_id) {
                            Some(block.id)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Uuid>>(),
            );
        }

        blocks
    }

    pub fn get_servers(&self) -> &HashMap<SharedString, ServerStates> {
        &self.servers
    }

    /// Get a block's server with its uuid.
    /// Default to return the local server.
    pub fn get_servers_by_block_id(&self, block_id: &Uuid) -> (SharedString, ServerStates) {
        for (name, server) in self.servers.iter() {
            if server.blocks.contains_key(block_id) {
                return (name.to_owned(), server.to_owned());
            }
        }

        let local_server_name = SharedString::new(LOCAL_SERVER_NAME);
        (
            local_server_name.clone(),
            self.servers.get(&local_server_name).unwrap().to_owned(),
        )
    }

    /// Get the active server.
    /// Return the local server if there are no remote ones.
    pub fn get_active_server(&self) -> (SharedString, ServerStates) {
        let active_server: &ServerStates = self.servers.get(&self.active_server).unwrap();
        (self.active_server.clone(), active_server.clone())
    }

    pub fn set_active_remote_server(&mut self, server_name: SharedString) {
        self.active_server = server_name;
    }

    pub fn set_search_scope(&mut self, search_scope: SearchScope) {
        self.search_scope = search_scope;
    }

    pub fn get_search_scope(&self) -> SearchScope {
        self.search_scope
    }

    pub fn get_search_scope_index(&self) -> usize {
        let mut selected_index = 0;

        for (index, item) in SEARCH_SCOPES_ENUMS.iter().enumerate() {
            if *item == self.search_scope {
                selected_index = index;
            }
        }

        selected_index
    }
}
