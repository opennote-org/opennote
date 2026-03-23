use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // MetadataSettings
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
            .await?;
        
        manager
            .create_table(
                Table::create()
                    .table(Blocks::Table)
                    .if_not_exists()
                    .col(pk_uuid(Blocks::Id).primary_key().not_null())
                    .col(string_null(Blocks::ParentId))
                    .col(boolean(Blocks::IsDeleted).not_null().default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blocks_parent_id")
                            .from(Blocks::Table, Blocks::ParentId)
                            .to(Blocks::Table, Blocks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Payloads::Table)
                    .if_not_exists()
                    .col(pk_uuid(Payloads::Id).primary_key().not_null())
                    .col(string(Payloads::BlockId).not_null())
                    .col(big_integer(Payloads::OrderRow).not_null())
                    .col(big_integer(Payloads::OrderColumn).not_null())
                    .col(big_integer(Payloads::CreatedAt).not_null())
                    .col(big_integer(Payloads::LastModified).not_null())
                    .col(text(Payloads::Texts))
                    .col(binary(Payloads::Bytes))
                    .col(json(Payloads::Vector))
                    .col(string(Payloads::ContentType).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payloads_block_id")
                            .from(Payloads::Table, Payloads::BlockId)
                            .to(Blocks::Table, Blocks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for performance
        manager
            .create_index(
                Index::create()
                    .table(Payloads::Table)
                    .col(Payloads::BlockId)
                    .name("index_payloads_block_id")
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Payloads::Table)
                    .col(Payloads::CreatedAt)
                    .name("index_payloads_created_at")
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Blocks::Table)
                    .col(Blocks::ParentId)
                    .name("index_blocks_parent_id")
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Blocks::Table)
                    .col(Blocks::IsDeleted)
                    .name("index_blocks_is_deleted")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Payloads::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Blocks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MetadataSettings::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum MetadataSettings {
    Table,
    Id,
    EmbedderModelInUse,
    EmbedderModelVectorSizeInUse,
    VectorDatabaseInUse,
}

#[derive(DeriveIden)]
enum Blocks {
    Table,
    Id,
    ParentId,
    IsDeleted,
}

#[derive(DeriveIden)]
enum Payloads {
    Table,
    Id,
    BlockId,
    OrderRow,
    OrderColumn,
    CreatedAt,
    LastModified,
    ContentType,
    Texts,
    Bytes,
    Vector,
}
