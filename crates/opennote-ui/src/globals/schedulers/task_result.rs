use gpui::SharedString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// It stores the task results
#[derive(Debug, Deserialize, Serialize)]
pub struct TaskResult {
    /// An id that can be corresponded to the TaskInformation
    pub id: Uuid,

    /// true means succeeded, vice versa
    pub status: bool,

    /// A message from the executed task
    pub message: SharedString,

    /// Task results packed in a serde_json::Value
    pub data: Option<Value>,
}

impl TaskResult {
    pub fn new(
        id: Uuid,
        status: bool,
        message: impl Into<SharedString>,
        data: Option<Value>,
    ) -> Self {
        Self {
            id,
            status,
            message: message.into(),
            data,
        }
    }
}
