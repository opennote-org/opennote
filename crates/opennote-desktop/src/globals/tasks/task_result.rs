use gpui::SharedString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, Eq)]
pub enum TaskType {
    /// Tasks that haven't yet been categorized
    Uncategorized,
    /// Chunk block
    ChunkBlock(Uuid),
    /// Update n blocks
    UpdateNBlocks,
}

/// It stores the task results
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct TaskResult {
    /// An id that can be corresponded to the TaskInformation
    pub id: Uuid,

    /// true means succeeded, vice versa
    pub status: bool,

    /// A message from the executed task
    pub message: SharedString,

    /// The type of this task. Useful when listening for results.
    pub task_type: TaskType,

    /// Task results packed in a serde_json::Value
    pub data: Option<Value>,
}

impl TaskResult {
    pub fn new(
        id: Uuid,
        status: bool,
        message: impl Into<SharedString>,
        task_type: TaskType,
        data: Option<Value>,
    ) -> Self {
        Self {
            id,
            status,
            message: message.into(),
            task_type,
            data,
        }
    }
}
