use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use gpui::Global;

use crate::globals::schedulers::{
    task_information::DebounceTaskInformation, task_result::TaskResult,
};

/// It tracks the execution results of async debounce tasks
pub struct DebounceTaskScheduler {
    pub tasks: Vec<DebounceTaskInformation>,
    pub results: Vec<TaskResult>,
    pub timer: Option<Instant>,
    pub debounce: Duration,
}

impl Global for DebounceTaskScheduler {}

impl DebounceTaskScheduler {
    /// Once called, the scheduler will check if the debounce has hit.
    /// If hit, it will execute the tasks.
    ///
    /// This is usually executed intermittenly by any component that cares about
    /// the task execution, like the notification center.
    ///
    /// Return true if the task has executed, vice versa.
    pub fn ping(&mut self) -> bool {
        if let Some(timer) = self.timer {
            let has_debounce_reached = timer.elapsed() >= self.debounce;

            // If the timer hits, we start deduplicate,
            // Then we excute all of them
            if has_debounce_reached {
                // Push (identifier, task) to HashMap
                // Overwrite if identifier is equal
                let mut to_execute = HashMap::new();
                for task in std::mem::take(&mut self.tasks) {
                    to_execute.insert(task.task_identifier.clone(), task);
                }

                // Execute the final items left in the HashMap
                for (_, task) in to_execute {
                    task.task.detach();
                }

                // Reset the timer after an execution
                self.timer = Some(Instant::now());

                return true;
            }
        }

        false
    }

    /// Register a debounce task.
    /// It will also check if the tasks are good to execute.
    pub fn register(&mut self, task_information: DebounceTaskInformation) {
        // If this is the first task, we set a timer and register
        if self.timer.is_none() {
            self.timer = Some(Instant::now());
        }

        // Once task is registered, we will need to check the timer
        if self.ping() {
            return;
        }

        // if the time does not hit, we register the task then return
        self.tasks.push(task_information);
    }

    pub fn get_all_task_results(&mut self) -> Vec<TaskResult> {
        std::mem::take(&mut self.results)
    }
}
