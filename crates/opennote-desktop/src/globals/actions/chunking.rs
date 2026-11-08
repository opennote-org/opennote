use gpui_kit::Window;

use opennote_core_logics::payload::build_payloads;
use opennote_models::{block::Block, payload::Payload};

use crate::globals::{
    language::LanguageProfile,
    tasks::{
        helpers::start_task, task_information::TaskInformation, task_result::TaskType,
        unique_notifications::ChunkBlockNotification,
    },
};

pub fn chunk_block(window: &mut Window, cx: &mut gpui_kit::App, mut block: Block, text: String) {
    let language_profile = LanguageProfile::get_language_profile(cx).unwrap();
    let chunking_message = language_profile.get_description("chunking_block").unwrap();
    let completed_message = language_profile
        .get_description("chunking_completed")
        .unwrap();
    let failed_message = language_profile.get_description("chunking_failed").unwrap();

    let task = TaskInformation::new(
        chunking_message,
        TaskType::ChunkBlock { block_id: block.id },
        true,
    );

    start_task::<ChunkBlockNotification>(
        cx,
        window,
        task,
        async move |cx, _window_handle| {
            // Chunk in the background
            let payloads: Vec<Payload> = cx
                .background_executor()
                .spawn(async move {
                    let payloads = match build_payloads(block.id, text, None) {
                        Ok(results) => results,
                        Err(error) => {
                            tracing::error!("Error when trying to save a document: {}", error);
                            vec![]
                        }
                    };

                    Ok::<Vec<Payload>, anyhow::Error>(payloads)
                })
                .await?;

            block.payloads = payloads;

            // Scheduler should receive the result at this point
            Ok(Some(serde_json::to_value(block).unwrap()))
        },
        |_cx| {},
        move || completed_message.clone(),
        move |error| failed_message.replace("{}", &error.to_string()).into(),
    );
}
