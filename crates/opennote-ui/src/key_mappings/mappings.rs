//! This file defines mappings between the generally defined keyboard shortcuts
//! and the UI actions, as well as UI actions

use anyhow::Result;
use gpui::{Action, actions};

use crate::key_mappings::key_contexts::{EDITOR, GENERAL, SIDEBAR, WORKSPACE};

actions!(
    workspace,
    [ToggleSidebar, ToggleSearchBar, ToggleCommandBar]
);
actions!(sidebar, [CreateOneBlock, DeleteBlocks]);
actions!(general, [MoveUp, MoveDown, MoveLeft, MoveRight]);
actions!(editor, [SaveDocument]);

/// You will also need to add the action in `crates/opennote-models/src/configurations/key_mappings.rs`
pub fn into_action(context: &str, action: &str) -> Result<Box<dyn Action>> {
    match context {
        WORKSPACE => match action {
            "ToggleSidebar" => Ok(Box::new(ToggleSidebar)),
            "ToggleSearchBar" => Ok(Box::new(ToggleSearchBar)),
            "ToggleCommandBar" => Ok(Box::new(ToggleCommandBar)),
            "CreateOneBlock" => Ok(Box::new(CreateOneBlock)),
            _ => Err(anyhow::anyhow!(
                "Unknown action for context '{}': {}",
                context,
                action
            )),
        },
        SIDEBAR => match action {
            "DeleteBlocks" => Ok(Box::new(DeleteBlocks)),
            _ => Err(anyhow::anyhow!(
                "Unknown action for context '{}': {}",
                context,
                action
            )),
        },
        GENERAL => match action {
            "MoveUp" => Ok(Box::new(MoveUp)),
            "MoveDown" => Ok(Box::new(MoveDown)),
            "MoveLeft" => Ok(Box::new(MoveLeft)),
            "MoveRight" => Ok(Box::new(MoveRight)),
            _ => Err(anyhow::anyhow!(
                "Unknown action for context '{}': {}",
                context,
                action
            )),
        },
        EDITOR => match action {
            "SaveDocument" => Ok(Box::new(SaveDocument)),
            _ => Err(anyhow::anyhow!(
                "Unknown action for context '{}': {}",
                context,
                action
            )),
        },
        _ => Err(anyhow::anyhow!(
            "Unknown action for context '{}': {}",
            context,
            action
        )),
    }
}
