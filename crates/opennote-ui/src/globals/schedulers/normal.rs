use gpui::{App, AsyncApp, Global};

use crate::globals::schedulers::{
    task_information::TaskInformation,
    task_result::{TaskResult, TaskType},
};

/// It tracks the execution results of async tasks
pub struct NormalTaskScheduler {
    pub tasks: Vec<TaskInformation>,
    pub results: Vec<TaskResult>,
}

impl Global for NormalTaskScheduler {}

impl NormalTaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            results: Vec::new(),
        }
    }

    pub fn init(cx: &mut App) {
        cx.set_global(NormalTaskScheduler::new());
    }

    pub fn has_pending_items(&self) -> bool {
        !self.tasks.is_empty() || !self.results.is_empty()
    }

    /// Specify a task type to check if that kind of task has pending results.
    /// Input None to get task results of all kinds.
    pub fn has_pending_task_results(&self, task_type: Option<TaskType>) -> bool {
        if let Some(task_type) = task_type {
            for result in self.results.iter() {
                if result.task_type == task_type {
                    return true;
                }
            }
        }

        false
    }

    /// Register a task for getting its results back
    pub fn register(&mut self, task_information: TaskInformation) {
        self.tasks.push(task_information);
    }

    pub fn register_result(&mut self, task_result: TaskResult) {
        self.results.push(task_result);
    }

    /// Get all tasks and then deplete
    pub fn get_all_tasks(&mut self) -> Vec<TaskInformation> {
        std::mem::take(&mut self.tasks)
    }

    /// Get all uncategorized task results and then deplete
    pub fn get_uncategorized_task_results(&mut self) -> Vec<TaskResult> {
        let mut uncategorized = Vec::new();
        let mut pointer = 0;

        while pointer < self.results.len() {
            // Remove the matched result
            match self.results[pointer].task_type {
                TaskType::Uncategorized => {
                    uncategorized.push(self.results.remove(pointer));
                    // No increment here.
                    // Index has shifted after remove.
                }
                _ => {
                    // Increment the pointer number when it is not matched
                    pointer += 1;
                }
            };
        }

        uncategorized
    }

    /// Get a specific task
    pub fn get_task_result(&mut self, task_type: TaskType) -> Option<TaskResult> {
        let mut result_index = None;
        for (index, result) in self.results.iter().enumerate() {
            if result.task_type == task_type {
                result_index = Some(index);
            }
        }

        if let Some(index) = result_index {
            return Some(self.results.remove(index));
        }

        None
    }
}

pub fn register_task(cx: &mut AsyncApp, task: TaskInformation) {
    let _ = cx.update_global::<NormalTaskScheduler, ()>(|this, _cx| {
        this.register(task);
    });
}

pub fn register_result(cx: &mut AsyncApp, task_result: TaskResult) {
    let _ = cx.update_global::<NormalTaskScheduler, ()>(|this, _cx| {
        this.register_result(task_result);
    });
}
