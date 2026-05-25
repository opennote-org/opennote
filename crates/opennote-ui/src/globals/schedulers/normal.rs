use gpui::Global;

use crate::globals::schedulers::{task_information::TaskInformation, task_result::TaskResult};

/// It tracks the execution results of async tasks
pub struct NormalTaskScheduler {
    pub tasks: Vec<TaskInformation>,
    pub results: Vec<TaskResult>,
}

impl Global for NormalTaskScheduler {}

impl NormalTaskScheduler {
    pub fn register(&mut self, task_information: TaskInformation) {
        self.tasks.push(task_information);
    }

    pub fn get_all_task_results(&mut self) -> Vec<TaskResult> {
        std::mem::take(&mut self.results)
    }
}
