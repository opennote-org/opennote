pub mod env_vars;

/*
 * Shared between different ends (desktop, server, etc)
 */
pub const CONFIGURATIONS_FILE_NAME: &str = "configurations.json";
pub const DATA_STORAGE_FOLDER_NAME: &str = "data";
pub const VECTOR_DATABASE_FILENAME: &str = "vector_database";
pub const SQLITE_VECTOR_DATABASE_FILE_EXTENSION: &str = "sqlite";

/*
 * Desktop Only
 */
pub const APP_DATA_FOLDER_NAME: &str = "opennote";
pub const LOCAL_SERVER_NAME: &str = "local";

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
