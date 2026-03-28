//! Track down the status of each task

use std::{collections::HashMap, vec};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub enum TaskStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub status: TaskStatus,
    pub message: Option<String>,
}

impl TaskRecord {
    pub fn new() -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            status: TaskStatus::InProgress,
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksScheduler {
    pub registered_tasks: Vec<TaskRecord>,
    pub generated_results: HashMap<String, Value>,
}

impl TasksScheduler {
    pub fn new() -> Self {
        Self {
            registered_tasks: vec![],
            generated_results: HashMap::new(),
        }
    }

    pub fn create_new_task(&mut self) -> String {
        let task: TaskRecord = TaskRecord::new();
        self.registered_tasks.push(task.clone());

        task.task_id
    }

    pub fn search_by_task_id(&self, task_id: &str) -> Option<&TaskRecord> {
        self.registered_tasks
            .iter()
            .find(|task| task.task_id == task_id)
    }

    /// Reserved for future uses
    #[allow(dead_code)]
    pub fn delete_by_task_id(&mut self, task_id: &str) -> bool {
        if let Some(pos) = self
            .registered_tasks
            .iter()
            .position(|task| task.task_id == task_id)
        {
            self.registered_tasks.remove(pos);
            return true;
        }

        false
    }

    pub fn update_status_by_task_id(
        &mut self,
        task_id: &str,
        new_status: TaskStatus,
        message: Option<String>,
    ) -> bool {
        if let Some(task) = self
            .registered_tasks
            .iter_mut()
            .find(|task| task.task_id == task_id)
        {
            task.status = new_status;
            task.message = message;
            return true;
        }

        false
    }

    pub fn set_status_to_complete(&mut self, task_id: &str, data: Value) -> bool {
        if let Some(task) = self
            .registered_tasks
            .iter_mut()
            .find(|task| task.task_id == task_id)
        {
            task.status = TaskStatus::Completed;
            self.generated_results.insert(task_id.to_string(), data);
            return true;
        }

        false
    }

    pub fn get_task_result(&mut self, task_id: &str) -> Option<Value> {
        return self.generated_results.remove(task_id);
    }
}
