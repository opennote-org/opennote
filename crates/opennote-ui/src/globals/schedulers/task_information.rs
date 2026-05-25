use gpui::{SharedString, Task};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stores a debounce task
pub struct DebounceTaskInformation {
    /// An id that can be corresponded to the TaskResult
    pub id: Uuid,

    /// Identify whether multiple tasks are the same.
    /// Useful when scheduling a debounce task for deduplication.
    pub task_identifier: SharedString,

    /// Store all unexecuted tasks until debounce reaches
    pub task: Task<Result<(), anyhow::Error>>,

    /// A message to display to the UI
    pub message: SharedString,
}

impl DebounceTaskInformation {
    pub fn new(
        task: Task<Result<(), anyhow::Error>>,
        task_identifier: impl Into<SharedString>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            task,
            task_identifier: task_identifier.into(),
            message: message.into(),
        }
    }
}

/// It stores the task information
#[derive(Debug, Deserialize, Serialize)]
pub struct TaskInformation {
    /// An id that can be corresponded to the TaskResult
    pub id: Uuid,

    /// A message to display to the UI
    pub message: SharedString,
}

impl TaskInformation {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            id: Uuid::new_v4(),
            message: message.into(),
        }
    }
}
