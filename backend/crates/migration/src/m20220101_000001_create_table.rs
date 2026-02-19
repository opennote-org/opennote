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
                    .to_owned(),
            )
            .await?;

        // Insert default metadata settings
        let db = manager.get_connection();
        db.execute_unprepared(
            "INSERT OR IGNORE INTO metadata_settings (id, embedder_model_in_use, embedder_model_vector_size_in_use) VALUES (1, '', 0)"
        ).await?;

        // Collections
        manager
            .create_table(
                Table::create()
                    .table(Collections::Table)
                    .if_not_exists()
                    .col(string(Collections::Id).primary_key())
                    .col(string(Collections::Title))
                    .col(integer(Collections::CreatedAt))
                    .col(integer(Collections::LastModified))
                    .to_owned(),
            )
            .await?;

        // Documents
        manager
            .create_table(
                Table::create()
                    .table(Documents::Table)
                    .if_not_exists()
                    .col(string(Documents::Id).primary_key())
                    .col(string(Documents::CollectionMetadataId))
                    .col(string(Documents::Title))
                    .col(integer(Documents::CreatedAt))
                    .col(integer(Documents::LastModified))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_documents_collection_metadata_id") // `fk` stands for foreign key
                            .from(Documents::Table, Documents::CollectionMetadataId)
                            .to(Collections::Table, Collections::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // DocumentChunks
        manager
            .create_table(
                Table::create()
                    .table(DocumentChunks::Table)
                    .if_not_exists()
                    .col(string(DocumentChunks::Id).primary_key())
                    .col(string(DocumentChunks::DocumentMetadataId))
                    .col(string(DocumentChunks::CollectionMetadataId))
                    .col(string(DocumentChunks::Content))
                    .col(ColumnDef::new(DocumentChunks::DenseTextVector).binary().not_null())
                    .col(integer(DocumentChunks::ChunkOrder))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_document_chunks_document_metadata_id")
                            .from(DocumentChunks::Table, DocumentChunks::DocumentMetadataId)
                            .to(Documents::Table, Documents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_document_chunks_document_metadata_id_chunk_order")
                            .table(DocumentChunks::Table)
                            .col(DocumentChunks::DocumentMetadataId)
                            .col(DocumentChunks::ChunkOrder)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        // Users
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(string(Users::Id).primary_key())
                    .col(string(Users::Username).unique_key())
                    .col(string(Users::Password))
                    .col(string(Users::Configuration))
                    .to_owned(),
            )
            .await?;

        // UserResources
        manager
            .create_table(
                Table::create()
                    .table(UserResources::Table)
                    .if_not_exists()
                    .col(string(UserResources::UserId))
                    .col(string(UserResources::ResourceIds))
                    .primary_key(
                        Index::create()
                            .name("pk_user_resources") // `pk` stands for primary key
                            .col(UserResources::UserId)
                            .col(UserResources::ResourceIds),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_resources_user_id")
                            .from(UserResources::Table, UserResources::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(UserResources::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Users::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(DocumentChunks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Documents::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Collections::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(MetadataSettings::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum MetadataSettings {
    Table,
    Id,
    EmbedderModelInUse,
    EmbedderModelVectorSizeInUse,
}

#[derive(Iden)]
enum Collections {
    Table,
    Id,
    Title,
    CreatedAt,
    LastModified,
}

#[derive(Iden)]
enum Documents {
    Table,
    Id,
    CollectionMetadataId,
    Title,
    CreatedAt,
    LastModified,
}

#[derive(Iden)]
enum DocumentChunks {
    Table,
    Id,
    DocumentMetadataId,
    CollectionMetadataId,
    Content,
    DenseTextVector,
    ChunkOrder,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Username,
    Password,
    Configuration,
}

#[derive(Iden)]
enum UserResources {
    Table,
    UserId,
    ResourceIds,
}
