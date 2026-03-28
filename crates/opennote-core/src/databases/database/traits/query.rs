use sea_orm::Condition;

pub trait DataQueryFilter {
    fn get_database_filter(&self) -> Option<Condition>;
}