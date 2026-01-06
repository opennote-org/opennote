//! The entrypoint of importing data from various sources. 
//! Currently, we are going to incorporate the following:
//! 1. Webpage
//! 2. Text file

pub mod responses;
pub mod requests;
pub mod models;
pub mod traits;

/// Connectors
pub mod webpage;
pub mod text_file;
pub mod relationship_database;