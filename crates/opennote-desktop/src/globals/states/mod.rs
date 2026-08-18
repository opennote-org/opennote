pub mod server_registry;

use std::collections::HashMap;

use gpui::{App, AppContext, Global, SharedString, WeakEntity, WindowId};
use uuid::Uuid;

use opennote_core_logics::helpers::run_async_code;
use opennote_data::{Databases, search::SearchScope};
use opennote_models::{
    block::Block, configurations::fields::remote_server::RemoteServerConfiguration,
    constants::LOCAL_SERVER_NAME, query::BlockQuery,
};

use crate::{
    globals::{
        actions::route_helpers::route_read_blocks,
        bootstrap::{GlobalApplicationBootStrap, SEARCH_SCOPES_ENUMS},
        states::server_registry::{ServerRegistry, ServerStates},
    },
    widgets::pane::Pane,
};

/// It manages general global states
pub struct States {
    /// States of the remote servers
    servers: ServerRegistry,

    /// The active server for each workspace.
    /// The key is a WindowId.
    active_servers: HashMap<WindowId, SharedString>,

    /// The pane that is active for each workspace.
    /// The key is a WindowId.
    pub active_panes: HashMap<WindowId, WeakEntity<Pane>>,

    pub search_scope: SearchScope,
}

impl Global for States {}

impl States {
    pub fn new(servers: HashMap<String, RemoteServerConfiguration>) -> Self {
        Self {
            active_servers: HashMap::new(),
            servers: ServerRegistry::build_servers(servers),
            active_panes: HashMap::new(),
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

    pub fn get_active_pane(&self, cx: &App) -> Option<WeakEntity<Pane>> {
        let Some(active_window_handle) = cx.active_window() else {
            return None;
        };

        self.active_panes
            .get(&active_window_handle.window_id())
            .cloned()
    }

    /// Overwrite the existing blocks of a server in the states with the new blocks
    pub fn hard_update_blocks(&mut self, server_name: &SharedString, blocks: Vec<Block>) {
        if let Some(server) = self.servers.get_servers_mut().get_mut(server_name) {
            server.blocks = HashMap::from_iter(blocks.into_iter().map(|item| (item.id, item)));
        }
    }

    /// It will refresh blocks across all servers
    pub fn refresh_blocks_list(&self, cx: &mut App) {
        let servers = self.get_servers().to_owned();
        let databases = cx.read_global::<GlobalApplicationBootStrap, Databases>(|this, _cx| {
            this.0.databases.clone()
        });

        for (name, server) in servers {
            let databases = databases.clone();
            cx.spawn(async move |cx| {
                let (server_name, results) = match route_read_blocks(
                    &name,
                    &server,
                    &databases,
                    &BlockQuery::All,
                    false,
                    false,
                )
                .await
                {
                    Ok(results) => (name, Ok(results)),
                    Err(error) => {
                        log::error!("{}", error);
                        (name, Err(error))
                    }
                };

                if let Ok(blocks) = results {
                    match cx.update_global::<States, ()>(|this, _cx| {
                        this.hard_update_blocks(&server_name, blocks);
                    }) {
                        Ok(_) => {}
                        Err(error) => log::error!("{}", error),
                    }
                }
            })
            .detach();
        }
    }

    pub fn get_block(&self, block_id: &Uuid) -> Option<Block> {
        for (_name, server) in self.get_servers().iter() {
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

        for (_name, server) in self.get_servers().iter() {
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
        for (name, server) in self.get_servers().iter() {
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

        for (_name, server) in self.get_servers().iter() {
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

    pub fn get_servers(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, HashMap<SharedString, ServerStates>> {
        self.servers.get_servers()
    }

    /// Cheap clone the ServerRegistry
    pub fn get_server_registry(&self) -> ServerRegistry {
        self.servers.clone()
    }

    /// Default to return the local server only.
    pub fn get_servers_by_block_ids(
        &self,
        block_ids: &Vec<Uuid>,
    ) -> Vec<(SharedString, ServerStates)> {
        let mut involved_servers = Vec::new();

        for (name, server) in self.get_servers().iter() {
            let any_block_id_contained = block_ids
                .iter()
                .any(|item| server.blocks.contains_key(item));

            if any_block_id_contained {
                involved_servers.push((name.to_owned(), server.to_owned()));
            }
        }

        involved_servers
    }

    /// Get the active server for a window.
    /// Return the local server if the window has no active server yet.
    pub fn get_active_server(&self, window_id: WindowId) -> (SharedString, ServerStates) {
        let active_server_name = self.get_active_server_name(window_id);
        let active_server: ServerStates = self
            .get_servers()
            .get(&active_server_name)
            .unwrap()
            .to_owned();
        (active_server_name, active_server.clone())
    }

    pub fn get_active_server_name(&self, window_id: WindowId) -> SharedString {
        self.active_servers
            .get(&window_id)
            .cloned()
            .unwrap_or_else(|| SharedString::new(LOCAL_SERVER_NAME))
    }

    pub fn set_active_server(&mut self, window_id: WindowId, server_name: SharedString) {
        self.active_servers.insert(window_id, server_name);
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

    pub fn update_servers(&mut self, servers: HashMap<String, RemoteServerConfiguration>) {
        self.servers = ServerRegistry::build_servers(servers);
    }
}
