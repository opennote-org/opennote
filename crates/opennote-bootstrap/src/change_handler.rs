use anyhow::Result;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::{configurations::system::SystemConfigurations, metadata::Metadata};

pub async fn handle_changes(
    system_configurations: &SystemConfigurations,
    databases: &Databases,
    embedder_entry: &EmbedderEntry,
    metadata: &Metadata,
) -> Result<()> {
    let changes = metadata.detect_changes(system_configurations);

    // Database is more fundamental.
    // If the database has changed, the vector database needs to be reset.
    if changes.database_changed {
        databases
            .vector_database
            .reset_index(
                &system_configurations.vector_database.index,
                system_configurations.embedder.dimensions,
            )
            .await?;

        return Ok(());
    }

    // We just need to reindex the vector database,
    // if only the vector database and the embedding model have changed,
    // because the vector database is a branch of the database.
    if changes.vector_database_changed || changes.embedding_model_changed {
        databases
            .vector_database
            .reindex_documents(system_configurations, &databases.database, embedder_entry)
            .await?;
    }

    Ok(())
}
