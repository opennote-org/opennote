use sea_orm_migration::{prelude::*, schema::*};

use crate::m20220101_000001_create_table::MetadataSettings;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MetadataSettings::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new("vector_database_in_use")
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MetadataSettings::Table)
                    .drop_column("vector_database_in_use")
                    .to_owned(),
            )
            .await
    }
}
