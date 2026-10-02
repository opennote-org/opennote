use anyhow::{Error, Result};
use gpui_kit::{AnyWindowHandle, App, AsyncApp, SharedString, Window};

use crate::globals::tasks::{
    task_information::TaskInformation,
    task_result::TaskResult,
    tracker::{register_long_running_completion, register_long_running_task},
};

/// Start a fire-and-forget task running in the background.
/// It will send a notification to indicate whether it is still runing.
/// It will also send a notification in the frontend on completion or failure.
pub fn start_task<T>(
    cx: &mut App,
    window: &mut Window,
    task: TaskInformation,
    business_logics: impl AsyncFnOnce(&mut AsyncApp, AnyWindowHandle) -> Result<()> + 'static,
    refresh_logics: impl Fn(&mut AsyncApp) + 'static,
    message_on_success: impl Fn() -> SharedString + 'static,
    message_on_error: impl Fn(Error) -> SharedString + 'static,
) where
    T: 'static,
{
    let window = window.window_handle();

    cx.spawn(async move |cx| {
        let task_id = task.id;
        let task_type = task.task_type;

        // Register task in the tracker
        register_long_running_task::<T>(window, cx, task);

        match business_logics(cx, window).await {
            Ok(_) => {}
            Err(error) => {
                register_long_running_completion::<T>(
                    window,
                    cx,
                    TaskResult::new(task_id, false, message_on_error(error), task_type, None),
                );

                return;
            }
        }

        register_long_running_completion::<T>(
            window,
            cx,
            TaskResult::new(task_id, true, message_on_success(), task_type, None),
        );

        refresh_logics(cx)
    })
    .detach();
}
