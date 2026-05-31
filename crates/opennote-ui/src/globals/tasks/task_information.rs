use gpui::{SharedString, Task};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::globals::tasks::task_result::TaskType;

/// It stores the task information
#[derive(Debug, Deserialize, Serialize)]
pub struct TaskInformation {
    /// An id that can be corresponded to the TaskResult
    pub id: Uuid,

    /// Task type
    pub task_type: TaskType,

    /// Does this task require long running
    pub is_long_running_task: bool,

    /// A message to display to the UI
    pub message: SharedString,
}

impl TaskInformation {
    pub fn new(
        message: impl Into<SharedString>,
        task_type: TaskType,
        is_long_running_task: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            is_long_running_task,
            message: message.into(),
        }
    }
}
