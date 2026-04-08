pub mod database;
pub mod search;
pub mod vector_database;

use std::sync::Arc;

use anyhow::Result;

use opennote_models::configurations::system::SystemConfigurations;

use crate::{
    database::{shared::create_database, traits::database::Database},
    vector_database::{shared::create_vector_database, traits::VectorDatabase},
};

/// At the moment, it only abstracts database-related logics of documents and chunks.
///
/// Users, collections, configurations and on are not yet needed for further abstractions
#[derive(Clone)]
pub struct Databases {
    pub database: Arc<dyn Database>,
    pub vector_database: Arc<dyn VectorDatabase>,
}

impl Databases {
    pub async fn new(config: &SystemConfigurations) -> Result<Self> {
        let vector_database = create_vector_database(config).await?;
        let database = create_database(config).await?;

        Ok(Self {
            database,
            vector_database,
        })
    }
}
