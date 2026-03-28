//! The entrypoint of importing data from various sources.
//! Currently, we are going to incorporate the following:
//! 1. Webpage
//! 2. Text file

pub mod models;
pub mod requests;
pub mod responses;
pub mod traits;

pub mod relationship_database;
pub mod text_file;
/// Connectors
pub mod webpage;
