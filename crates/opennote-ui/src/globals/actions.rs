use gpui::AppContext;
use opennote_core_logics::{
    block::{create_blocks, delete_blocks, read_blocks, update_blocks},
    payload::{PayloadContentParameters, build_payload},
};
use opennote_data::{Databases, database::enums::BlockQuery};
use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};
use opennote_models::{block::Block, configurations::system::VectorDatabaseConfig};
use uuid::Uuid;

use crate::globals::{
    bootstrap::GlobalApplicationBootStrap, helpers::get_language_profile, states::States,
};

/// It will create one new block with a default title payload
pub fn create_one_block(app_cx: &mut gpui::App, parent_block_id: Option<Uuid>) {
    app_cx
        .spawn(async move |cx| {
            log::debug!("Creating 1 block...");

            let (default_block_title, databases, embedders, vector_database_config) =
                cx.read_global::<GlobalApplicationBootStrap, (
                    String,
                    Databases,
                    Option<EmbedderEntry>,
                    VectorDatabaseConfig,
                )>(|this, cx| {
                    let language_profile = get_language_profile(cx.global(), cx.global()).unwrap();

                    (
                        language_profile.default_block_title.clone(),
                        this.0.databases.clone(),
                        this.0.embedders.clone(),
                        this.0.configurations.system.vector_database.clone(),
                    )
                })?;

            let mut block = Block::new(parent_block_id, Vec::new());

            let payload = build_payload(
                block.id,
                PayloadContentParameters {
                    title: Some(default_block_title.to_string()),
                    ..Default::default()
                },
            )?;

            match &embedders {
                Some(embedders) => {
                    let mut vectorized_payloads =
                        send_vectorization(vec![payload], &embedders).await?;

                    if let Some(vectorized_payload) = vectorized_payloads.pop() {
                        block.payloads.push(vectorized_payload);
                    }
                }
                None => {
                    log::error!(
                        "No embedders available. Please load an embedder before proceeding"
                    );
                    return Err(anyhow::anyhow!("No embedders available"));
                }
            }

            let num_blocks =
                match create_blocks(&vector_database_config, &databases, vec![block]).await {
                    Ok(result) => result.len(),
                    Err(error) => {
                        log::error!("{}", error);
                        return Err(error);
                    }
                };

            log::debug!(
                "Block creation finished for {} blocks, preceed to refreshing the block list...",
                num_blocks
            );

            let _ = cx.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

/// It will create one new block with a default title payload
pub fn delete_n_blocks(app_cx: &mut gpui::App, block_ids: Vec<Uuid>) {
    app_cx
        .spawn(async move |cx| {
            log::debug!("Deleting {} blocks...", block_ids.len());

            let (databases, vector_database_config) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, VectorDatabaseConfig)>(
                    |this, _cx| {
                        (
                            this.0.databases.clone(),
                            this.0.configurations.system.vector_database.clone(),
                        )
                    },
                )?;

            match delete_blocks(&databases, &vector_database_config, block_ids).await {
                Ok(_) => {}
                Err(error) => {
                    log::error!("{}", error);
                    return Err(error);
                }
            }

            log::debug!("Blocks deletion finished, preceed to refreshing the block list...");

            let _ = cx.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}

pub fn update_parent(app_cx: &mut gpui::App, new_parent_block_id: Uuid, block_ids: Vec<Uuid>) {
    log::debug!("Updating blocks' parent...");

    app_cx.read_global::<GlobalApplicationBootStrap, ()>(|this, app| {
        let databases = this.0.databases.clone();
        let vector_database_config = this.0.configurations.system.vector_database.clone();

        app.spawn(async move |app| {
            match read_blocks(&databases, &BlockQuery::ByIds(block_ids)).await {
                Ok(blocks) => {
                    let blocks = blocks
                        .into_iter()
                        .map(|mut item| {
                            item.parent_id = Some(new_parent_block_id);
                            item
                        })
                        .collect();

                    let _ = update_blocks(&vector_database_config, &databases, blocks).await;
                }
                Err(error) => log::error!("{}", error),
            };

            log::debug!(
                "Blocks parent id update finished, preceed to refreshing the block list..."
            );

            let _ = app.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });
        })
        .detach();
    });
}
