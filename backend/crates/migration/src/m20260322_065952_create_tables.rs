use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Blocks::Table)
                    .if_not_exists()
                    .col(string(Blocks::Id).primary_key())
                    .col(string_null(Blocks::ParentId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Flags::Table)
                    .if_not_exists()
                    .col(pk_auto(Flags::Id)) // This is not included in the Flag
                    .col(string(Flags::BlockId).not_null())
                    .col(boolean(Flags::IsDeleted).default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_flags_block_id")
                            .from(Flags::Table, Flags::BlockId)
                            .to(Blocks::Table, Blocks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Payloads::Table)
                    .if_not_exists()
                    .col(pk_auto(Payloads::Id))
                    .col(string(Payloads::BlockId).not_null())
                    .col(string_null(Payloads::Title))
                    .col(big_integer(Payloads::OrderRow))
                    .col(big_integer(Payloads::OrderColumn))
                    .col(big_integer(Payloads::CreatedAt))
                    .col(big_integer(Payloads::LastModified))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payloads_block_id")
                            .from(Payloads::Table, Payloads::BlockId)
                            .to(Blocks::Table, Blocks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Contents::Table)
                    .if_not_exists()
                    .col(pk_auto(Contents::Id))
                    .col(integer(Contents::PayloadId).not_null())
                    .col(binary(Contents::Value))
                    .col(json(Contents::Vector))
                    .col(string(Contents::ContentType))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_contents_payload_id")
                            .from(Contents::Table, Contents::PayloadId)
                            .to(Payloads::Table, Payloads::Id)
                            .on_delete(ForeignKeyAction::Cascade),
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
                    .table(Contents::Table)
                    .col(Contents::PayloadId)
                    .name("index_contents_payload_id")
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Flags::Table)
                    .col(Flags::BlockId)
                    .name("index_flags_block_id")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Contents::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Payloads::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Flags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Blocks::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Blocks {
    Table,
    Id,
    ParentId,
}

#[derive(DeriveIden)]
enum Flags {
    Table,
    Id,
    BlockId,
    IsDeleted,
}

#[derive(DeriveIden)]
enum Payloads {
    Table,
    Id,
    BlockId,
    Title,
    OrderRow,
    OrderColumn,
    CreatedAt,
    LastModified,
}

#[derive(DeriveIden)]
enum Contents {
    Table,
    Id,
    PayloadId,
    Value,
    Vector,
    ContentType,
}
