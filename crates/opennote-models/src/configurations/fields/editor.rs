//! Persistent app preferences and the preferences window.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const DEFAULT_THEME_ID: &str = "velotype";
const DEFAULT_LANGUAGE_ID: &str = "en-US";

/// Where pasted clipboard images should be stored before inserting Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImagePasteBehavior {
    None,
    CopyToOpenNoteAssetsFolder,
}

impl ImagePasteBehavior {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CopyToOpenNoteAssetsFolder => "copy_to_opennote_assets_folder",
        }
    }
}

/// User preferences persisted under the app config directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorConfigurations {
    pub default_language_id: String,
    pub default_theme_id: String,
    pub show_table_headers: bool,
    pub image_paste_behavior: ImagePasteBehavior,
    pub keybindings: BTreeMap<String, Vec<String>>,
}

impl Default for EditorConfigurations {
    fn default() -> Self {
        Self {
            default_language_id: DEFAULT_LANGUAGE_ID.into(),
            default_theme_id: DEFAULT_THEME_ID.into(),
            show_table_headers: true,
            image_paste_behavior: ImagePasteBehavior::CopyToOpenNoteAssetsFolder,
            keybindings: BTreeMap::new(),
        }
    }
}
