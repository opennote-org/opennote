//! This file defines mappings between the generally defined keyboard shortcuts
//! and the UI actions, as well as UI actions

use anyhow::Result;
use gpui::{Action, actions};

use crate::key_mappings::key_contexts::{SIDEBAR, WORKSPACE};

actions!(workspace, [ToggleSidebar, ToggleSearchBar]);
actions!(sidebar, [CreateOneBlock]);

pub fn into_action(context: &str, action: &str) -> Result<Box<dyn Action>> {
    match context {
        WORKSPACE => match action {
            "ToggleSidebar" => Ok(Box::new(ToggleSidebar)),
            "ToggleSearchBar" => Ok(Box::new(ToggleSearchBar)),
            "CreateOneBlock" => Ok(Box::new(CreateOneBlock)),
            _ => Err(anyhow::anyhow!(
                "Unknown action for context '{}': {}",
                context,
                action
            )),
        },
        SIDEBAR => match action {
            "CreateOneBlock" => Ok(Box::new(CreateOneBlock)),
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
