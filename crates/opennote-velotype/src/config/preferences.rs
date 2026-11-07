//! Persistent app preferences and the preferences window.

use std::{path::PathBuf, str::FromStr};

use gpui::*;

/// Runtime-accessible editor settings mirrored from [`AppPreferences`] so the
/// render path can read them without touching disk. Toggling persists the new
/// value back to the preferences file.
pub struct EditorSettings {
    show_table_headers: bool,
    assets_storage_path: PathBuf,
}

impl Global for EditorSettings {}

impl EditorSettings {
    pub fn new(show_table_headers: bool, assets_storage_path: &str) -> Self {
        Self {
            show_table_headers,
            assets_storage_path: PathBuf::from_str(assets_storage_path).unwrap(),
        }
    }

    pub fn init(cx: &mut App, show_table_headers: bool, assets_storage_path: &str) {
        Self::set_global(cx, show_table_headers, assets_storage_path);
    }

    fn set_global(cx: &mut App, show_table_headers: bool, assets_storage_path: &str) {
        cx.set_global(Self::new(show_table_headers, assets_storage_path));
    }

    /// Whether table top rows are styled as headers. Defaults to `true` when
    /// the global has not been installed (e.g. in unit tests).
    pub fn show_table_headers(cx: &App) -> bool {
        cx.try_global::<Self>()
            .map(|settings| settings.show_table_headers)
            .unwrap_or(true)
    }

    pub fn get_assets_storage_path(&self) -> PathBuf {
        self.assets_storage_path.clone()
    }
}
