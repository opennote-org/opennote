use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct KeyCombination {
    pub key: String,
    
    pub following_keys: Vec<String>,
    
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct GlobalActions {
    pub open_config: Option<KeyCombination>,
    pub open_search: Option<KeyCombination>,
    pub toggle_sidebar: Option<KeyCombination>,
    pub switch_tab_next: Option<KeyCombination>,
    pub switch_tab_previous: Option<KeyCombination>,
    pub refresh: Option<KeyCombination>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct EditorNormalActions {
    pub cursor_move_left: Option<KeyCombination>,
    pub cursor_move_right: Option<KeyCombination>,
    pub cursor_move_up: Option<KeyCombination>,
    pub cursor_move_down: Option<KeyCombination>,
    pub enter_insert_mode: Option<KeyCombination>,
    pub enter_visual_mode: Option<KeyCombination>,
    pub save_document: Option<KeyCombination>,
    pub move_word_forward: Option<KeyCombination>,
    pub move_word_backward: Option<KeyCombination>,
    pub enter_visual_line_mode: Option<KeyCombination>,
    pub delete_line: Option<KeyCombination>,
    pub yank_line: Option<KeyCombination>,
    pub yank: Option<KeyCombination>,
    pub undo: Option<KeyCombination>,
    pub redo: Option<KeyCombination>,
    pub goto_end_of_document: Option<KeyCombination>,
    pub goto_beginning_of_document: Option<KeyCombination>,
    pub scroll_down_half_page: Option<KeyCombination>,
    pub scroll_up_half_page: Option<KeyCombination>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct EditorVisualActions {
    pub cursor_move_left: Option<KeyCombination>,
    pub cursor_move_right: Option<KeyCombination>,
    pub cursor_move_up: Option<KeyCombination>,
    pub cursor_move_down: Option<KeyCombination>,
    pub yank_selection: Option<KeyCombination>,
    pub delete_selection: Option<KeyCombination>,
    pub exit_visual_mode: Option<KeyCombination>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct EditorInsertActions {
    pub exit_insert_mode: Option<KeyCombination>,
    pub cursor_move_left: Option<KeyCombination>,
    pub cursor_move_right: Option<KeyCombination>,
    pub cursor_move_up: Option<KeyCombination>,
    pub cursor_move_down: Option<KeyCombination>,
    pub delete_left: Option<KeyCombination>,
    pub delete_right: Option<KeyCombination>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct KeyProfile {
    pub global: GlobalActions,
    pub editor_normal: EditorNormalActions,
    pub editor_visual: EditorVisualActions,
    pub editor_insert: EditorInsertActions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct KeyMappingConfiguration {
    pub is_vim_key_mapping_enabled: bool,
    pub vim_profile: KeyProfile,
    pub conventional_profile: KeyProfile,
}

impl KeyCombination {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            following_keys: vec![],
            modifiers: vec![],
        }
    }

    pub fn new_with_following_keys(key: &str, following_keys: Vec<String>) -> Self {
        Self {
            key: key.to_string(),
            following_keys,
            modifiers: vec![],
        }
    }

    pub fn with_modifiers(key: &str, modifiers: Vec<Modifier>) -> Self {
        Self {
            key: key.to_string(),
            following_keys: vec![],
            modifiers,
        }
    }
}

impl KeyProfile {
    pub fn vim_default() -> Self {
        Self {
            global: GlobalActions {
                open_config: Some(KeyCombination::with_modifiers(",", vec![Modifier::Meta])),
                open_search: Some(KeyCombination::new("/")),
                toggle_sidebar: Some(KeyCombination::with_modifiers("b", vec![Modifier::Meta])),
                switch_tab_next: Some(KeyCombination::with_modifiers("k", vec![Modifier::Shift])),
                switch_tab_previous: Some(KeyCombination::with_modifiers(
                    "j",
                    vec![Modifier::Shift],
                )),
                refresh: Some(KeyCombination::with_modifiers("r", vec![Modifier::Meta])),
            },
            editor_normal: EditorNormalActions {
                cursor_move_left: Some(KeyCombination::new("h")),
                cursor_move_right: Some(KeyCombination::new("l")),
                cursor_move_up: Some(KeyCombination::new("k")),
                cursor_move_down: Some(KeyCombination::new("j")),
                enter_insert_mode: Some(KeyCombination::new("i")),
                enter_visual_mode: Some(KeyCombination::new("v")),
                save_document: Some(KeyCombination::with_modifiers("s", vec![Modifier::Meta])),
                move_word_forward: Some(KeyCombination::new("e")),
                move_word_backward: Some(KeyCombination::new("b")),
                enter_visual_line_mode: Some(KeyCombination::with_modifiers(
                    "v",
                    vec![Modifier::Shift],
                )),
                delete_line: Some(KeyCombination::new("d")),
                yank_line: Some(KeyCombination::new("y")),
                yank: Some(KeyCombination::new("y")),
                undo: Some(KeyCombination::new("u")),
                redo: Some(KeyCombination::with_modifiers("r", vec![Modifier::Ctrl])),
                goto_end_of_document: Some(KeyCombination::with_modifiers(
                    "g",
                    vec![Modifier::Shift],
                )),
                goto_beginning_of_document: Some(KeyCombination::new_with_following_keys(
                    "g",
                    vec!["g".to_string()],
                )),
                scroll_down_half_page: Some(KeyCombination::with_modifiers(
                    "d",
                    vec![Modifier::Ctrl],
                )),
                scroll_up_half_page: Some(KeyCombination::with_modifiers(
                    "u",
                    vec![Modifier::Ctrl],
                )),
            },
            editor_visual: EditorVisualActions {
                cursor_move_left: Some(KeyCombination::new("h")),
                cursor_move_right: Some(KeyCombination::new("l")),
                cursor_move_up: Some(KeyCombination::new("k")),
                cursor_move_down: Some(KeyCombination::new("j")),
                yank_selection: Some(KeyCombination::new("y")),
                delete_selection: Some(KeyCombination::new("d")),
                exit_visual_mode: Some(KeyCombination::new("Escape")),
            },
            editor_insert: EditorInsertActions {
                exit_insert_mode: Some(KeyCombination::new("Escape")),
                cursor_move_left: None,
                cursor_move_right: None,
                cursor_move_up: None,
                cursor_move_down: None,
                delete_left: None,
                delete_right: None,
            },
        }
    }

    pub fn conventional_default() -> Self {
        Self {
            global: GlobalActions {
                open_config: Some(KeyCombination::with_modifiers(",", vec![Modifier::Meta])),
                open_search: Some(KeyCombination::with_modifiers("p", vec![Modifier::Meta])),
                toggle_sidebar: Some(KeyCombination::with_modifiers("b", vec![Modifier::Meta])),
                switch_tab_next: Some(KeyCombination::with_modifiers("S", vec![Modifier::Ctrl])),
                switch_tab_previous: Some(KeyCombination::with_modifiers(
                    "A",
                    vec![Modifier::Ctrl, Modifier::Shift],
                )),
                refresh: Some(KeyCombination::with_modifiers("r", vec![Modifier::Meta])),
            },
            editor_normal: EditorNormalActions {
                cursor_move_left: None,
                cursor_move_right: None,
                cursor_move_up: None,
                cursor_move_down: None,
                enter_insert_mode: None,
                enter_visual_mode: None,
                save_document: Some(KeyCombination::with_modifiers("s", vec![Modifier::Meta])),
                move_word_forward: None,
                move_word_backward: None,
                enter_visual_line_mode: None,
                delete_line: None,
                yank_line: None,
                yank: None,
                undo: None,
                redo: None,
                goto_end_of_document: None,
                goto_beginning_of_document: None,
                scroll_down_half_page: None,
                scroll_up_half_page: None,
            },
            editor_visual: EditorVisualActions {
                cursor_move_left: Some(KeyCombination::new("ArrowLeft")),
                cursor_move_right: Some(KeyCombination::new("ArrowRight")),
                cursor_move_up: Some(KeyCombination::new("ArrowUp")),
                cursor_move_down: Some(KeyCombination::new("ArrowDown")),
                yank_selection: Some(KeyCombination::with_modifiers("c", vec![Modifier::Meta])),
                delete_selection: Some(KeyCombination::with_modifiers("x", vec![Modifier::Meta])),
                exit_visual_mode: Some(KeyCombination::new("Escape")),
            },
            editor_insert: EditorInsertActions {
                exit_insert_mode: None,
                cursor_move_left: Some(KeyCombination::new("ArrowLeft")),
                cursor_move_right: Some(KeyCombination::new("ArrowRight")),
                cursor_move_up: Some(KeyCombination::new("ArrowUp")),
                cursor_move_down: Some(KeyCombination::new("ArrowDown")),
                delete_left: Some(KeyCombination::new("Backspace")),
                delete_right: Some(KeyCombination::new("Delete")),
            },
        }
    }
}

impl Default for KeyMappingConfiguration {
    fn default() -> Self {
        Self {
            is_vim_key_mapping_enabled: false,
            vim_profile: KeyProfile::vim_default(),
            conventional_profile: KeyProfile::conventional_default(),
        }
    }
}
