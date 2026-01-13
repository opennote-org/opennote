use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Ctrl,
    Space, 
    CommandOrWindows, 
    OptionOrAlt, 
    Shift, 
    Enter, 
    Backspace, 
    FnKeys(String), 
    Arrows(String), 
    Others(String), 
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct KeyMapping {
    pub cursor_move_left: Key,
    pub cursor_move_right: Key,
    pub cursor_move_up: Key,
    pub cursor_move_down: Key,
    pub normal_mode: Key,
    pub insert_mode: Key,
    pub visual_mode: Key,
    pub open_configurations_popup: Key,
    pub open_search_popup: Key,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, JsonSchema)]
pub struct KeyMappingConfigurations {
    pub vim_key_mappings: KeyMapping,
    pub conventional_key_mappings: KeyMapping,
}