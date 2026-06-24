use gpui::{AnyWindowHandle, App, AppContext, AsyncApp, Global};
use gpui_component::{
    WindowExt,
    notification::{Notification, NotificationType},
};
use uuid::Uuid;

use crate::globals::tasks::{
    task_information::TaskInformation,
    task_result::{TaskResult, TaskType},
};

/// It tracks the execution results of async tasks
pub struct TaskTracker {
    pub tasks: Vec<TaskInformation>,
    pub results: Vec<TaskResult>,
}

impl Global for TaskTracker {}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            results: Vec::new(),
        }
    }

    pub fn init(cx: &mut App) {
        cx.set_global(TaskTracker::new());
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

    pub fn remove_task_by_id(&mut self, id: Uuid) {
        self.tasks.retain(|item| item.id == id);
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

pub fn register_task(window: AnyWindowHandle, cx: &mut AsyncApp, task: TaskInformation) {
    let message = task.message.clone();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.register(task);
    });

    let _ = cx.update_window(window, |_view, window, cx| {
        window.push_notification((NotificationType::Info, message), cx);
    });
}

pub fn register_long_running_task<T: 'static>(
    window: AnyWindowHandle,
    cx: &mut AsyncApp,
    task: TaskInformation,
) {
    let message = task.message.clone();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.register(task);
    });

    let _ = cx.update_window(window, |_view, window, cx| {
        window.push_notification(Notification::info(message).id::<T>().autohide(false), cx);
    });
}

/// It will remove the task information, then register the result
pub fn register_result(window: AnyWindowHandle, cx: &mut AsyncApp, task_result: TaskResult) {
    let notification_type = get_notification_type(task_result.status);
    let message = task_result.message.clone();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.remove_task_by_id(task_result.id);
        this.register_result(task_result);
    });

    let _ = cx.update_window(window, |_view, window, cx| {
        window.push_notification((notification_type, message), cx);
    });
}

/// It will remove the task information, then register the result
pub fn register_long_running_result<T: Sized + 'static>(
    window: AnyWindowHandle,
    cx: &mut AsyncApp,
    task_result: TaskResult,
) {
    let notification_type = get_notification_type(task_result.status);
    let message = task_result.message.clone();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.remove_task_by_id(task_result.id);
        this.register_result(task_result);
    });

    let _ = cx.update_window(window, |_view, window, cx| {
        window.remove_notification::<T>(cx);
        window.push_notification((notification_type, message), cx);
    });
}

fn get_notification_type(status: bool) -> NotificationType {
    match status {
        true => NotificationType::Success,
        false => NotificationType::Error,
    }
}
