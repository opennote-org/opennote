use gpui::{App, AsyncApp, Global};

use crate::globals::schedulers::{task_information::TaskInformation, task_result::TaskResult};

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

    /// Get all task results and then deplete
    pub fn get_all_task_results(&mut self) -> Vec<TaskResult> {
        std::mem::take(&mut self.results)
    }
}

pub fn register_task(cx: &mut AsyncApp, task: TaskInformation) {
    let _ = cx.update_global::<NormalTaskScheduler, ()>(|this, _cx| {
        this.register(task);
    });
}

pub fn register_result(cx: &mut AsyncApp, task_result: TaskResult) {
    let _ = cx.update_global::<NormalTaskScheduler, ()>(|this, cx| {
        this.register_result(task_result);
    });
}
