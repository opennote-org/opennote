use gpui::{BorrowAppContext, Context, SharedString, Subscription, Window};
use gpui_component::{
    WindowExt,
    notification::{Notification, NotificationType},
};

use crate::globals::tasks::{
    task_information::TaskInformation,
    task_result::{TaskResult, TaskType},
    tracker::TaskTracker,
    unique_notifications::ChunkBlockNotification,
};

/// TODO:
/// - Add debounce to ChunkBlock and upate block tasks
pub struct NotificationCenter {
    _subscriptions: Vec<Subscription>,
}

impl NotificationCenter {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        let mut _subscriptions = Vec::new();

        // Get updates from the normal task scheduler
        _subscriptions.push(
            cx.observe_global_in::<TaskTracker>(window, |_this, window, cx| {
                let scheduler: &TaskTracker = cx.global();
                if !scheduler.has_pending_items() {
                    return;
                }

                let (task_results, task_information) = cx
                    .update_global::<TaskTracker, (Vec<TaskResult>, Vec<TaskInformation>)>(
                        |this, _cx| (this.get_uncategorized_task_results(), this.get_all_tasks()),
                    );

                if !task_results.is_empty() {
                    for result in task_results {
                        let notification_type = match result.status {
                            true => NotificationType::Success,
                            false => NotificationType::Error,
                        };
                        window.push_notification((notification_type, result.message), cx);
                    }
                }

                if !task_information.is_empty() {
                    for task in task_information {
                        if task.is_long_running_task {
                            match task.task_type {
                                TaskType::ChunkBlock(_) => {
                                    Self::push_unique_notifications::<ChunkBlockNotification>(
                                        cx,
                                        window,
                                        task.message,
                                    );
                                    continue;
                                }
                                _ => {}
                            }
                        }

                        window.push_notification((NotificationType::Info, task.message), cx);
                    }
                }
            }),
        );

        Self { _subscriptions }
    }

    /// An unique notification won't auto-hide.
    /// It needs to be manually hidden from the code.
    /// This is usually used for long running tasks.
    fn push_unique_notifications<T: 'static>(
        cx: &mut Context<Self>,
        window: &mut Window,
        message: SharedString,
    ) {
        window.push_notification(Notification::info(message).id::<T>().autohide(false), cx);
    }
}
