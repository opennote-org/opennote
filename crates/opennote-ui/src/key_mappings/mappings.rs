//! This file defines mappings between the generally defined keyboard shortcuts
//! and the UI actions, as well as UI actions

use anyhow::Result;
use gpui::{Action, actions};

use crate::key_mappings::key_contexts::{GENERAL, SIDEBAR, WORKSPACE};

actions!(workspace, [ToggleSidebar, ToggleSearchBar]);
actions!(sidebar, [CreateOneBlock, DeleteBlocks]);
actions!(general, [MoveUp, MoveDown, MoveLeft, MoveRight]);

pub fn into_action(context: &str, action: &str) -> Result<Box<dyn Action>> {
    match context {
        WORKSPACE => match action {
            "ToggleSidebar" => Ok(Box::new(ToggleSidebar)),
            "ToggleSearchBar" => Ok(Box::new(ToggleSearchBar)),
            _ => Err(anyhow::anyhow!(
                "Unknown action for context '{}': {}",
                context,
                action
            )),
        },
        SIDEBAR => match action {
            "CreateOneBlock" => Ok(Box::new(CreateOneBlock)),
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
        _ => Err(anyhow::anyhow!(
            "Unknown action for context '{}': {}",
            context,
            action
        )),
    }
}
