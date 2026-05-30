use gpui::{BorrowAppContext, Context, Subscription, Window};
use gpui_component::{WindowExt, notification::NotificationType};

use crate::globals::schedulers::{
    normal::NormalTaskScheduler, task_information::TaskInformation, task_result::TaskResult,
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
        _subscriptions.push(cx.observe_global_in::<NormalTaskScheduler>(
            window,
            |_this, window, cx| {
                let scheduler: &NormalTaskScheduler = cx.global();
                if !scheduler.has_pending_items() {
                    return;
                }

                let (task_results, task_information) = cx
                    .update_global::<NormalTaskScheduler, (Vec<TaskResult>, Vec<TaskInformation>)>(
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
                        window.push_notification((NotificationType::Info, task.message), cx);
                    }
                }
            },
        ));

        Self { _subscriptions }
    }
}
