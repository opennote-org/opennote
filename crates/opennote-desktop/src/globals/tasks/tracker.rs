use std::collections::{HashMap, HashSet};

use gpui::{AnyWindowHandle, App, AppContext, AsyncApp, Global, Subscription, WindowId};
use gpui_component::{
    WindowExt,
    notification::{Notification, NotificationType},
};
use uuid::Uuid;

use crate::globals::tasks::{
    task_information::TaskInformation,
    task_result::{TaskResult, TaskType},
};

#[derive(Default)]
struct WindowTaskState {
    tasks: Vec<TaskInformation>,
    results: Vec<TaskResult>,
}

/// It tracks the execution results of async tasks, grouped by their owning window.
pub struct TaskTracker {
    windows: HashMap<WindowId, WindowTaskState>,
    window_closed_subscription: Option<Subscription>,
}

impl Global for TaskTracker {}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_closed_subscription: None,
        }
    }

    pub fn init(cx: &mut App) {
        cx.set_global(TaskTracker::new());

        let subscription = cx.on_window_closed(|cx| {
            let open_windows: HashSet<WindowId> = cx
                .windows()
                .iter()
                .map(|window| window.window_id())
                .collect();
            cx.global_mut::<TaskTracker>()
                .windows
                .retain(|window_id, _| open_windows.contains(window_id));
        });
        cx.global_mut::<TaskTracker>().window_closed_subscription = Some(subscription);
    }

    pub fn has_pending_items(&self, window_id: WindowId) -> bool {
        self.windows
            .get(&window_id)
            .is_some_and(|state| !state.tasks.is_empty() || !state.results.is_empty())
    }

    /// Specify a task type to check if that kind of task has pending results.
    /// Input None to check for task results of all kinds in the window.
    pub fn has_pending_task_results(
        &self,
        window_id: WindowId,
        task_type: Option<TaskType>,
    ) -> bool {
        let Some(state) = self.windows.get(&window_id) else {
            return false;
        };

        task_type.map_or(!state.results.is_empty(), |task_type| {
            state
                .results
                .iter()
                .any(|result| result.task_type == task_type)
        })
    }

    /// Register a task for getting its results back.
    pub fn register(&mut self, window_id: WindowId, task_information: TaskInformation) {
        self.windows
            .entry(window_id)
            .or_default()
            .tasks
            .push(task_information);
    }

    /// Remove a task and its result.
    /// Return false, if no task is found.
    fn complete(&mut self, window_id: WindowId, task_id: Uuid) -> Option<TaskInformation> {
        if let Some(state) = self.windows.get_mut(&window_id) {
            let task_index = state.tasks.iter().position(|task| task.id == task_id);

            match task_index {
                Some(result) => return Some(state.tasks.remove(result)),
                _ => return None,
            }
        }

        None
    }

    /// Complete a pending task and register its result in the same window.
    pub fn register_result(&mut self, window_id: WindowId, task_result: TaskResult) {
        if self.complete(window_id, task_result.id).is_some() {
            let Some(state) = self.windows.get_mut(&window_id) else {
                return;
            };

            state.results.push(task_result);
        }
    }

    /// Get the first matching task result from a window.
    pub fn get_task_result(
        &mut self,
        window_id: WindowId,
        task_type: TaskType,
    ) -> Option<TaskResult> {
        let (result, state_is_empty) = {
            let state = self.windows.get_mut(&window_id)?;
            let result_index = state
                .results
                .iter()
                .position(|result| result.task_type == task_type)?;
            let result = state.results.remove(result_index);
            let state_is_empty = state.tasks.is_empty() && state.results.is_empty();
            (result, state_is_empty)
        };

        if state_is_empty {
            self.windows.remove(&window_id);
        }

        Some(result)
    }
}

pub fn register_task(window: AnyWindowHandle, cx: &mut AsyncApp, task: TaskInformation) {
    let message = task.message.clone();
    let window_id = window.window_id();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.register(window_id, task);
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
    let window_id = window.window_id();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.register(window_id, task);
    });

    let _ = cx.update_window(window, |_view, window, cx| {
        window.push_notification(Notification::info(message).id::<T>().autohide(false), cx);
    });
}

/// It will remove the task information, then register the result
pub fn register_result(window: AnyWindowHandle, cx: &mut AsyncApp, task_result: TaskResult) {
    let notification_type = get_notification_type(task_result.status);
    let message = task_result.message.clone();
    let window_id = window.window_id();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.register_result(window_id, task_result);
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
    let window_id = window.window_id();

    let _ = cx.update_global::<TaskTracker, ()>(|this, _cx| {
        this.register_result(window_id, task_result);
    });

    let _ = cx.update_window(window, |_view, window, cx| {
        window.remove_notification::<T>(cx);
        window.push_notification((notification_type, message), cx);
    });
}

/// It will remove the task information, but never register the result.
pub fn register_long_running_completion<T: Sized + 'static>(
    window: AnyWindowHandle,
    cx: &mut AsyncApp,
    task_result: TaskResult,
) {
    let notification_type = get_notification_type(task_result.status);
    let message = task_result.message.clone();

    let window_id = window.window_id();
    let task_id = task_result.id;

    let _ = cx.update_global::<TaskTracker, ()>(|tracker, _cx| {
        tracker.complete(window_id, task_id);
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
