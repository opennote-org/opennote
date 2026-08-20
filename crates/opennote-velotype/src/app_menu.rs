//! Native application menu, app-level actions, and window close routing.
//!
//! This module owns menu construction and the actions that operate on the
//! active editor window. The Quit action is routed to the current window so the
//! existing unsaved-changes dialog remains authoritative for that window.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::components::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseWindow, ExportHtml, ExportPdf,
    InstallCliTool, NewWindow, NoRecentFiles, OpenFile, OpenPreferences, OpenRecentFile,
    QuitApplication, SaveDocument, SaveDocumentAs, SelectLanguage, SelectTheme, ShowAbout,
    ToggleWorkspace, UninstallCliTool,
};
use crate::config::{
    apply_configured_language, apply_configured_theme, import_language_config_and_select,
    import_theme_config_and_select, open_preferences_window, record_recent_file,
    remove_recent_file,
};
use crate::editor::{Editor, InfoDialogKind};
use crate::export::ExportFormat;
use crate::i18n::I18nManager;
use crate::window_chrome::velotype_window_options;

/// Global app-menu state for platform menu lifecycle hooks.
#[derive(Default)]
pub(crate) struct AppMenuState {
    window_closed_subscription: Option<Subscription>,
}

impl Global for AppMenuState {}

fn window_title(file_path: Option<&Path>) -> SharedString {
    if let Some(path) = file_path {
        // OsStr::to_string_lossy returns Cow<str>; calling .to_string() on
        // it always allocates a fresh String, even for the valid-UTF-8 path
        // (the common case). Borrow the Cow directly into format! — its
        // Display impl writes the borrowed bytes straight into the output
        // String, no intermediate allocation.
        format!(
            "Velotype - {}",
            path.file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_else(|| path.to_string_lossy())
        )
        .into()
    } else {
        SharedString::new("Velotype")
    }
}

/// Opens an editor window for the given Markdown content and optional path.
pub(crate) fn open_editor_window(
    cx: &mut App,
    markdown: String,
    file_path: Option<PathBuf>,
) -> WindowHandle<Editor> {
    let bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);
    let title = window_title(file_path.as_deref());
    let handle = cx
        .open_window(
            velotype_window_options(title, bounds),
            move |_window, cx| cx.new(move |cx| Editor::from_markdown(cx, markdown, file_path)),
        )
        .unwrap();

    handle
        .update(cx, |_editor, window, _cx| {
            window.activate_window();
        })
        .expect("newly opened editor window should be updateable");

    handle
}

pub(crate) fn open_file_in_new_window(cx: &mut App, path: &Path) -> anyhow::Result<()> {
    let markdown = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    open_editor_window(cx, markdown, Some(path.to_path_buf()));
    record_recent_file_and_refresh(path, cx);
    Ok(())
}

fn record_recent_file_and_refresh(path: &Path, cx: &mut App) {
    if let Err(err) = record_recent_file(path) {
        eprintln!("failed to update recent file history: {err}");
        return;
    }
    cx.refresh_windows();
}

#[allow(dead_code)]
pub(crate) fn record_recent_file_from_editor(path: &Path, cx: &mut App) {
    record_recent_file_and_refresh(path, cx);
}

fn show_window_prompt(window: Option<AnyWindowHandle>, title: &str, detail: &str, cx: &mut App) {
    if let Some(window) = window {
        let ok = cx.global::<I18nManager>().strings().info_dialog_ok.clone();
        let _ = window.update(cx, |_view, window, cx| {
            let buttons = [ok.as_str()];
            let _ = window.prompt(PromptLevel::Critical, title, Some(detail), &buttons, cx);
        });
    } else {
        eprintln!("{title}: {detail}");
    }
}

fn with_active_editor<R>(
    cx: &mut App,
    update: impl FnOnce(&mut Editor, &mut Window, &mut Context<Editor>) -> R,
) -> Option<R> {
    let window = cx.active_window()?.downcast::<Editor>()?;
    window.update(cx, update).ok()
}

fn show_info_dialog_on_active_editor(cx: &mut App, kind: InfoDialogKind) {
    let _ = with_active_editor(cx, move |editor, _window, cx| {
        editor.show_info_dialog(kind, cx);
    });
}

fn request_update_check_on_active_editor(cx: &mut App) {
    let _ = with_active_editor(cx, |editor, window, cx| {
        editor.request_check_updates(window, cx);
    });
}

fn open_recent_file(cx: &mut App, path: PathBuf) {
    let error_window = cx.active_window();
    open_recent_file_with_error_window(cx, path, error_window);
}

fn open_recent_file_with_error_window(
    cx: &mut App,
    path: PathBuf,
    error_window: Option<AnyWindowHandle>,
) {
    if !path.is_file() {
        if let Err(err) = remove_recent_file(&path) {
            eprintln!("failed to remove missing recent file: {err}");
        }
        cx.refresh_windows();
        let strings = cx.global::<I18nManager>().strings().clone();
        let detail = strings
            .recent_file_missing_message_template
            .replace("{path}", &path.to_string_lossy());
        show_window_prompt(
            error_window,
            &strings.recent_file_missing_title,
            &detail,
            cx,
        );
        return;
    }

    if let Err(err) = open_file_in_new_window(cx, &path) {
        let title = cx
            .global::<I18nManager>()
            .strings()
            .open_failed_title
            .clone();
        show_window_prompt(error_window, &title, &err.to_string(), cx);
    }
}

fn is_editor_scoped_menu_action(action: &dyn Action) -> bool {
    action.as_any().is::<SaveDocument>()
        || action.as_any().is::<SaveDocumentAs>()
        || action.as_any().is::<ExportHtml>()
        || action.as_any().is::<ExportPdf>()
        || action.as_any().is::<QuitApplication>()
        || action.as_any().is::<CloseWindow>()
        || action.as_any().is::<CheckForUpdates>()
        || action.as_any().is::<ShowAbout>()
        || action.as_any().is::<InstallCliTool>()
        || action.as_any().is::<UninstallCliTool>()
        || action.as_any().is::<ToggleWorkspace>()
}

fn is_window_context_menu_action(action: &dyn Action) -> bool {
    action.as_any().is::<NewWindow>()
        || action.as_any().is::<OpenFile>()
        || action.as_any().is::<OpenPreferences>()
        || action.as_any().is::<OpenRecentFile>()
        || action.as_any().is::<NoRecentFiles>()
        || action.as_any().is::<AddLanguageConfig>()
        || action.as_any().is::<AddThemeConfig>()
        || action.as_any().is::<InstallCliTool>()
        || action.as_any().is::<UninstallCliTool>()
        || is_editor_scoped_menu_action(action)
}

#[allow(dead_code)]
fn current_window_candidates(cx: &mut App) -> Vec<AnyWindowHandle> {
    let mut candidates = Vec::new();
    let mut push_unique = |window: AnyWindowHandle| {
        if candidates
            .iter()
            .all(|candidate: &AnyWindowHandle| candidate.window_id() != window.window_id())
        {
            candidates.push(window);
        }
    };

    if let Some(window) = cx.active_window() {
        push_unique(window);
    }
    if let Some(windows) = cx.window_stack() {
        for window in windows {
            push_unique(window);
        }
    }
    for window in cx.windows() {
        push_unique(window);
    }

    candidates
}

/// Executes one of the app-menu actions against the current application state.
pub(crate) fn dispatch_menu_action(action: &dyn Action, cx: &mut App) {
    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        prompt_and_open_files(cx);
    } else if action.as_any().is::<OpenPreferences>() {
        open_preferences_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        open_recent_file(cx, PathBuf::from(&action.path));
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompt_and_import_language_config(cx);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompt_and_import_theme_config(cx);
    } else if action.as_any().is::<ExportHtml>() {
        let _ = with_active_editor(cx, |editor, window, cx| {
            editor.export_document_via_prompt(ExportFormat::Html, window, cx)
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = with_active_editor(cx, |editor, window, cx| {
            editor.export_document_via_prompt(ExportFormat::Pdf, window, cx)
        });
    } else if let Some(action) = action.as_any().downcast_ref::<SelectTheme>() {
        match apply_configured_theme(cx, &action.theme_id) {
            Ok(changed) => {
                if changed {
                    cx.refresh_windows();
                }
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .preferences_save_failed_title
                    .clone();
                show_window_prompt(cx.active_window(), &title, &err.to_string(), cx);
            }
        }
    } else if let Some(action) = action.as_any().downcast_ref::<SelectLanguage>() {
        match apply_configured_language(cx, &action.language_id) {
            Ok(changed) => {
                if changed {
                    cx.refresh_windows();
                }
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .preferences_save_failed_title
                    .clone();
                show_window_prompt(cx.active_window(), &title, &err.to_string(), cx);
            }
        }
    } else if action.as_any().is::<CheckForUpdates>() {
        request_update_check_on_active_editor(cx);
    } else if action.as_any().is::<ShowAbout>() {
        show_info_dialog_on_active_editor(cx, InfoDialogKind::About);
    } else if action.as_any().is::<ToggleWorkspace>() {
        let _ = with_active_editor(cx, |editor, window, cx| {
            editor.toggle_workspace_drawer(window, cx);
        });
    }
}

/// Executes a menu action against a specific editor when the action is
/// editor-scoped, falling back to app-wide behavior for global actions.
pub(crate) fn dispatch_menu_action_for_editor(
    action: &dyn Action,
    target: &WeakEntity<Editor>,
    window: &mut Window,
    cx: &mut App,
) {
    if !is_window_context_menu_action(action) {
        let deferred_action = action.boxed_clone();
        cx.defer(move |cx| {
            dispatch_menu_action(deferred_action.as_ref(), cx);
        });
        return;
    }

    window.activate_window();
    let current_window = Some(window.window_handle());

    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        prompt_and_open_files_with_error_window(cx, current_window);
    } else if action.as_any().is::<OpenPreferences>() {
        open_preferences_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        open_recent_file_with_error_window(cx, PathBuf::from(&action.path), current_window);
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompt_and_import_language_config_with_error_window(cx, current_window);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompt_and_import_theme_config_with_error_window(cx, current_window);
    }
    // else if action.as_any().is::<SaveDocument>() {
    //     let _ = target.update(cx, |editor, cx| editor.request_save_document(cx));
    // } else if action.as_any().is::<SaveDocumentAs>() {
    //     let _ = target.update(cx, |editor, cx| editor.request_save_document_as(cx));
    // }
    else if action.as_any().is::<ExportHtml>() {
        let _ = target.update(cx, |editor, cx| {
            editor.export_document_via_prompt(ExportFormat::Html, window, cx);
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = target.update(cx, |editor, cx| {
            editor.export_document_via_prompt(ExportFormat::Pdf, window, cx);
        });
    } else if action.as_any().is::<CheckForUpdates>() {
        let _ = target.update(cx, |editor, cx| {
            editor.request_check_updates(window, cx);
        });
    } else if action.as_any().is::<ShowAbout>() {
        let _ = target.update(cx, |editor, cx| {
            editor.show_info_dialog(InfoDialogKind::About, cx)
        });
    } else if action.as_any().is::<ToggleWorkspace>() {
        let _ = target.update(cx, |editor, cx| {
            editor.toggle_workspace_drawer(window, cx);
        });
    }
}

fn prompt_and_open_files(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_open_files_with_error_window(cx, error_window);
}

fn prompt_and_open_files_with_error_window(cx: &mut App, error_window: Option<AnyWindowHandle>) {
    let prompt_title = cx
        .global::<I18nManager>()
        .strings()
        .open_markdown_files_prompt
        .clone();
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(prompt_title.into()),
    });

    cx.spawn(async move |cx| match prompt.await {
        Ok(Ok(Some(paths))) => {
            let _ = cx.update(move |cx| {
                for path in paths {
                    if let Err(err) = open_file_in_new_window(cx, &path) {
                        let title = cx
                            .global::<I18nManager>()
                            .strings()
                            .open_failed_title
                            .clone();
                        show_window_prompt(error_window, &title, &err.to_string(), cx);
                    }
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            let _ = cx.update(move |cx| {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .open_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &detail, cx);
            });
        }
        Ok(Ok(None)) | Err(_) => {}
    })
    .detach();
}

fn prompt_and_import_language_config(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_import_language_config_with_error_window(cx, error_window);
}

fn prompt_and_import_language_config_with_error_window(
    cx: &mut App,
    error_window: Option<AnyWindowHandle>,
) {
    let prompt_title = cx
        .global::<I18nManager>()
        .strings()
        .add_language_config_prompt
        .clone();
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(prompt_title.into()),
    });

    cx.spawn(async move |cx| match prompt.await {
        Ok(Ok(Some(paths))) => {
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = cx.update(move |cx| {
                let result = import_language_config_and_select(cx, &path);
                match result {
                    Ok(_) => {
                        cx.refresh_windows();
                    }
                    Err(err) => {
                        let title = cx
                            .global::<I18nManager>()
                            .strings()
                            .config_import_failed_title
                            .clone();
                        show_window_prompt(error_window, &title, &err.to_string(), cx);
                    }
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            let _ = cx.update(move |cx| {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .config_import_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &detail, cx);
            });
        }
        Ok(Ok(None)) | Err(_) => {}
    })
    .detach();
}

fn prompt_and_import_theme_config(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_import_theme_config_with_error_window(cx, error_window);
}

fn prompt_and_import_theme_config_with_error_window(
    cx: &mut App,
    error_window: Option<AnyWindowHandle>,
) {
    let prompt_title = cx
        .global::<I18nManager>()
        .strings()
        .add_theme_config_prompt
        .clone();
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(prompt_title.into()),
    });

    cx.spawn(async move |cx| match prompt.await {
        Ok(Ok(Some(paths))) => {
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = cx.update(move |cx| {
                let result = import_theme_config_and_select(cx, &path);
                match result {
                    Ok(_) => {
                        cx.refresh_windows();
                    }
                    Err(err) => {
                        let title = cx
                            .global::<I18nManager>()
                            .strings()
                            .config_import_failed_title
                            .clone();
                        show_window_prompt(error_window, &title, &err.to_string(), cx);
                    }
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            let _ = cx.update(move |cx| {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .config_import_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &detail, cx);
            });
        }
        Ok(Ok(None)) | Err(_) => {}
    })
    .detach();
}

fn handle_window_closed(cx: &mut App) {
    if cx.windows().is_empty() {
        cx.quit();
    }
}

/// Installs menu state, action handlers, and the native menu bar.
pub fn init(cx: &mut App) {
    cx.set_global(AppMenuState::default());
    let subscription = cx.on_window_closed(handle_window_closed);
    cx.global_mut::<AppMenuState>().window_closed_subscription = Some(subscription);

    cx.on_action(|_: &NewWindow, cx| {
        dispatch_menu_action(&NewWindow, cx);
    });
    cx.on_action(|_: &OpenFile, cx| {
        dispatch_menu_action(&OpenFile, cx);
    });
    cx.on_action(|_: &OpenPreferences, cx| {
        dispatch_menu_action(&OpenPreferences, cx);
    });
    cx.on_action(|action: &OpenRecentFile, cx| {
        dispatch_menu_action(action, cx);
    });
    cx.on_action(|_: &NoRecentFiles, cx| {
        dispatch_menu_action(&NoRecentFiles, cx);
    });
    cx.on_action(|_: &AddLanguageConfig, cx| {
        dispatch_menu_action(&AddLanguageConfig, cx);
    });
    cx.on_action(|_: &AddThemeConfig, cx| {
        dispatch_menu_action(&AddThemeConfig, cx);
    });
    cx.on_action(|_: &SaveDocument, cx| {
        dispatch_menu_action(&SaveDocument, cx);
    });
    cx.on_action(|_: &SaveDocumentAs, cx| {
        dispatch_menu_action(&SaveDocumentAs, cx);
    });
    cx.on_action(|_: &ExportHtml, cx| {
        dispatch_menu_action(&ExportHtml, cx);
    });
    cx.on_action(|_: &ExportPdf, cx| {
        dispatch_menu_action(&ExportPdf, cx);
    });
    cx.on_action(|action: &SelectTheme, cx| {
        dispatch_menu_action(action, cx);
    });
    cx.on_action(|action: &SelectLanguage, cx| {
        dispatch_menu_action(action, cx);
    });
    cx.on_action(|_: &CheckForUpdates, cx| {
        dispatch_menu_action(&CheckForUpdates, cx);
    });
    cx.on_action(|_: &ShowAbout, cx| {
        dispatch_menu_action(&ShowAbout, cx);
    });
    cx.on_action(|_: &ToggleWorkspace, cx| {
        dispatch_menu_action(&ToggleWorkspace, cx);
    });
    cx.on_action(|_: &QuitApplication, cx| {
        dispatch_menu_action(&QuitApplication, cx);
    });
    cx.on_action(|_: &CloseWindow, cx| {
        dispatch_menu_action(&CloseWindow, cx);
    });

    cx.activate(true);
}
