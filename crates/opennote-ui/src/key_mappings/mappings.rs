//! This file defines mappings between the generally defined keyboard shortcuts
//! and the UI actions, as well as UI actions

use anyhow::Result;
use gpui::{Action, actions};

actions!(workspace, [ToggleSidebar, ToggleSearchBar]);

pub fn into_action(context: &str, action: &str) -> Result<Box<dyn Action>> {
    match context {
        "workspace" => match action {
            "ToggleSidebar" => Ok(Box::new(ToggleSidebar)),
            "ToggleSearchBar" => Ok(Box::new(ToggleSearchBar)),
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
