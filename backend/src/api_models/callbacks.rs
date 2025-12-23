use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tasks_scheduler::TaskStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveTaskResultRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResponse {
    task_id: String,
    status: TaskStatus,
    message: Option<String>,
    data: Option<Value>,
}

impl GenericResponse {
    pub fn succeed(task_id: String, data: &impl Serialize) -> Self {
        Self {
            task_id,
            status: TaskStatus::Completed,
            message: None,
            data: Some(serde_json::to_value(data).unwrap()),
        }
    }

    pub fn in_progress(task_id: String) -> Self {
        Self {
            task_id,
            status: TaskStatus::InProgress,
            message: None,
            data: None,
        }
    }

    pub fn fail(task_id: String, message: String) -> Self {
        Self {
            task_id,
            status: TaskStatus::Failed,
            message: Some(message),
            data: None,
        }
    }
}
