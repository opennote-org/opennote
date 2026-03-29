use sea_orm::{ColumnTrait, Condition};
use uuid::Uuid;

use crate::database::traits::query::DataQueryFilter;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum PayloadQuery {
    All,                   // All blocks in the database
    ByIds(Vec<Uuid>),      // Specific payloads
    ByBlockIds(Vec<Uuid>), // By payloads' block ids
}

impl DataQueryFilter for PayloadQuery {
    fn get_database_filter(&self) -> Option<Condition> {
        use opennote_entities::payloads;

        match &self {
            PayloadQuery::All => Some(Condition::all()),
            PayloadQuery::ByIds(ids) => {
                if ids.is_empty() {
                    return None;
                }

                Some(
                    Condition::any()
                        .add(payloads::Column::Id.is_in(ids.iter().map(|item| item.to_string()))),
                )
            }
            PayloadQuery::ByBlockIds(block_ids) => {
                if block_ids.is_empty() {
                    return None;
                }

                Some(Condition::any().add(
                    payloads::Column::BlockId.is_in(block_ids.iter().map(|item| item.to_string())),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum BlockQuery {
    All,                   // All blocks in the database
    Root,                  // Blocks without parent
    ByIds(Vec<Uuid>),      // Specific blocks
    ChildrenOf(Vec<Uuid>), // By parent ids
}
