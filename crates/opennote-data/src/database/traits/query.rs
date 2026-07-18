use sea_orm::{ColumnTrait, Condition};

use opennote_models::query::PayloadQuery;

pub trait DataQueryFilter {
    fn get_database_filter(&self) -> Option<Condition>;
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
                    Condition::any().add(
                        payloads::Column::Id
                            .is_in(ids.iter().map(|item| sea_orm::Value::Uuid(Some(*item)))),
                    ),
                )
            }
            PayloadQuery::ByBlockIds(block_ids) => {
                if block_ids.is_empty() {
                    return None;
                }

                Some(
                    Condition::any().add(
                        payloads::Column::BlockId.is_in(
                            block_ids
                                .iter()
                                .map(|item| sea_orm::Value::Uuid(Some(*item))),
                        ),
                    ),
                )
            }
        }
    }
}
