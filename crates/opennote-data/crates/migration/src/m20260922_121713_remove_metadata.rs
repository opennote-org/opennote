use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260322_065952_create_tables::MetadataSettings;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MetadataSettings::Table).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MetadataSettings::Table)
                    .if_not_exists()
                    .col(integer(MetadataSettings::Id).primary_key().auto_increment())
                    .col(string(MetadataSettings::EmbedderModelInUse).default(""))
                    .col(integer(MetadataSettings::EmbedderModelVectorSizeInUse).default(0))
                    .col(string(MetadataSettings::VectorDatabaseInUse).default(""))
                    .to_owned(),
            )
            .await
    }
}
