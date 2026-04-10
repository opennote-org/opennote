use opennote_core_logics::{
    block::{create_blocks, update_blocks},
    payload::{PayloadContentParameters, create_payload},
};
use opennote_data::Databases;
use opennote_embedder::{entry::EmbedderEntry, vectorization::send_vectorization};

use crate::globals::{
    bootstrap::GlobalApplicationBootStrap, helpers::get_language_profile, states::States,
};

/// It will create one new block with a default title payload
pub fn create_one_block(app_cx: &mut gpui::App) {
    app_cx
        .spawn(async move |cx| {
            log::debug!("Creating 1 block...");

            let (default_block_title, databases, embedders) = cx
                .read_global::<GlobalApplicationBootStrap, (String, Databases, Option<EmbedderEntry>)>(
                    |this, cx| {
                        let language_profile =
                            get_language_profile(cx.global(), cx.global()).unwrap();

                        (language_profile.default_block_title.clone(), this.0.databases.clone(), this.0.embedders.clone())
                    },
                )?;

            let block = match create_blocks(&databases, 1).await {
                Ok(mut result) => result.pop(),
                Err(error) => {
                    log::error!("{}", error);
                    return Err(error);
                }
            };

            if let Some(mut block) = block {
                let payload = create_payload(
                    block.id,
                    PayloadContentParameters {
                        title: Some(default_block_title.to_string()),
                        ..Default::default()
                    },
                )?;

                match &embedders {
                    Some(embedders) => {
                        let mut vectorized_payloads =
                            send_vectorization(vec![payload], &embedders)
                                .await?;

                        if let Some(vectorized_payload) = vectorized_payloads.pop() {
                            block.payloads.push(vectorized_payload);
                        }
                    }
                    None => {
                        log::error!("No embedders available. Please load an embedder before proceeding");
                        return Err(anyhow::anyhow!("No embedders available"));
                    }
                }

                match update_blocks(&databases, vec![block]).await {
                    Ok(_) => {},
                    Err(error) => log::error!("Error when trying to update blocks: {}", error)
                }
            }

            log::debug!(
                "Block creation finished, preceed to refreshing the block list..."
            );

            let _ = cx.update_global::<States, ()>(|_this, cx| {
                States::refresh_blocks_list(cx);
            });

            Ok::<(), anyhow::Error>(())
        })
        .detach();
}
