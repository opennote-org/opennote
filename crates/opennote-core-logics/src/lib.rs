pub mod block;
pub mod payload;

use anyhow::Result;

use futures::future::join;
use uuid::Uuid;

use opennote_data::{
    Databases,
    database::enums::{BlockQuery, PayloadQuery},
};
use opennote_models::{block::Block, configurations::system::VectorDatabaseConfig};
