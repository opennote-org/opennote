pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260322_065952_create_tables::Migration),
            Box::new(m20260922_121713_remove_metadata::Migration),
        ]
    }
}
mod m20260322_065952_create_tables;
mod m20260922_121713_remove_metadata;
