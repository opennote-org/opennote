use gpui::{App, ParentElement, Window};

use gpui_component::{
    WindowExt,
    button::{Button, ButtonVariants},
    dialog::Dialog,
};

use crate::globals::helpers::get_language_profile;

pub const PENDING_TASKS_WARNING: &str = "pending_tasks_warning";

/// Builds a warning dialog using four keys derived from `language_profile_entry`:
/// `<entry>_title`, `<entry>_message`, `<entry>_cancel`, and `<entry>_discard`.
pub fn open_warning_dialogue(
    language_profile_entry: &'static str,
) -> impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static {
    move |dialogue, _window, cx| {
        let language_profile = get_language_profile(cx).unwrap();
        let title = language_profile[&format!("{language_profile_entry}_title")].clone();
        let message = language_profile[&format!("{language_profile_entry}_message")].clone();
        let cancel_string = language_profile[&format!("{language_profile_entry}_cancel")].clone();
        let discard_string = language_profile[&format!("{language_profile_entry}_discard")].clone();

        dialogue
            .title(title)
            .child(message)
            .footer(move |_, _, _, _| {
                vec![
                    Button::new("cancel-window-close")
                        .label(cancel_string.clone())
                        .on_click(|_, window, cx| {
                            window.close_dialog(cx);
                        }),
                    Button::new("discard-and-close-window")
                        .danger()
                        .label(discard_string.clone())
                        .on_click(|_, window, cx| {
                            // remove_window() bypasses on_window_should_close,
                            // preventing the confirmation dialog from reopening.
                            window.close_dialog(cx);
                            window.remove_window();
                        }),
                ]
            })
    }
}
