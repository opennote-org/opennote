//! This file defines mappings between the generally defined keyboard shortcuts
//! and the UI actions, as well as UI actions

use gpui::{Action, actions};
use anyhow::Result;

actions!(workspace_sidebar, [ToggleWorkspaceSidebar]);

pub fn into_action(context: &str, action: &str) -> Result<impl Action> {
    match context {
        "workspace_sidebar" => match action {
            "ToggleWorkspaceSidebar" => Ok(ToggleWorkspaceSidebar),
            _ => Err(anyhow::anyhow!("Unknown action for context '{}': {}", context, action))
        }
        _ => Err(anyhow::anyhow!("Unknown action for context '{}': {}", context, action))
    }
}