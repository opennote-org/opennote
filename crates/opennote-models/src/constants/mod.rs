pub mod env_vars;
pub mod hex;

/*
 * Shared between different ends (desktop, server, etc)
 */
pub const CONFIGURATIONS_FILE_NAME: &str = "configurations.json";
pub const DATA_STORAGE_FOLDER_NAME: &str = "data";
pub const VECTOR_DATABASE_FILENAME: &str = "vector_database";
pub const SQLITE_VECTOR_DATABASE_FILE_EXTENSION: &str = "sqlite";
pub const METADATA_FILENAME: &str = "metadata.json";

/*
 * Desktop Only
 */
pub const APP_DATA_FOLDER_NAME: &str = "opennote";
pub const LOCAL_SERVER_NAME: &str = "local";
pub const KEY_MAPPINGS_FILE_NAME: &str = "key_mappings.json";
pub const DESKTOP_APP_NAME: &str = "OpenNote";
pub const DESKTOP_TITLE_SEPARATOR: &str = " - ";
pub const DESKTOP_SETTINGS_PANEL_NAME: &str = "Settings";
pub const LOADING_WINDOW_WIDTH: f32 = 420.;
pub const LOADING_WINDOW_HEIGHT: f32 = 240.;
pub const DEFAULT_BLOCK_STATES_FILE_NAME: &str = "block_states.json";
pub const LOG_WINDOW_CAPACITY: usize = 2000;

/*
 * Server Only
 */
pub const SERVER_PASSWORD: &str = "";
pub const SERVER_DATA_FOLDER_NAME: &str = "opennote_server";
pub const ROOT_ENDPOINT: &str = "/api/v1";
pub const READ_WORKSPACE_BLOCKS_ENDPOINT: &str = "/read_workspace_blocks";
pub const CREATE_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/create_blocks_in_workspace";
pub const DELETE_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/delete_blocks_in_workspace";
pub const UPDATE_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/update_blocks_in_workspace";
pub const SEARCH_BLOCKS_IN_WORKSPACE_ENDPOINT: &str = "/search_blocks_in_workspace";
