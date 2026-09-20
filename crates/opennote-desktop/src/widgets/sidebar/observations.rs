use std::future::Ready;

use gpui::Context;

use crate::widgets::sidebar::OpenNoteSidebar;

use opennote_core_logics::configurations::{ApplicationType, get_configuration_folder_path};
use opennote_models::traits::LoadFromAndSaveToFile;

pub fn observe_on_app_quit_for_block_states_persistence(
    this: &mut OpenNoteSidebar,
    _cx: &mut Context<'_, OpenNoteSidebar>,
) -> Ready<()> {
    let configuration_folder_path = get_configuration_folder_path(ApplicationType::Desktop);

    if let Err(error) = this.block_states.save_to_file(&configuration_folder_path) {
        tracing::error!("Failed to save block states: {error:#}");
    }

    std::future::ready(())
}
