use gpui_kit::{App, Bounds, SharedString, TitlebarOptions, WindowBounds, WindowOptions, px, size};

use opennote_models::constants::{
    DESKTOP_APP_NAME, DESKTOP_TITLE_SEPARATOR, LOADING_WINDOW_HEIGHT, LOADING_WINDOW_WIDTH,
};

/// Create a main window option.
///
/// In OpenNote, a main window is the one that hosts a Workspace.
pub fn create_main_window_option(title: impl Into<SharedString>) -> WindowOptions {
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        focus: true,
        show: true,
        ..Default::default()
    }
}

pub fn create_loading_window_option(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(
        None,
        size(px(LOADING_WINDOW_WIDTH), px(LOADING_WINDOW_HEIGHT)),
        cx,
    );

    WindowOptions {
        titlebar: Some(TitlebarOptions {
            appears_transparent: matches!(std::env::consts::OS, "macos" | "windows"),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        focus: true,
        show: true,
        ..Default::default()
    }
}

pub fn create_window_option(
    cx: &App,
    title: impl Into<SharedString>,
    width: f32,
    height: f32,
) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(width), px(height)), cx);

    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        focus: true,
        show: true,
        ..Default::default()
    }
}

pub fn format_window_title(
    interface_name: Option<&str>,
    server_name: Option<&str>,
    document_name: Option<&str>,
) -> SharedString {
    let mut components = String::from(DESKTOP_APP_NAME);

    if let Some(name) = interface_name {
        components.push_str(DESKTOP_TITLE_SEPARATOR);
        components.push_str(name);
    }

    if let Some(name) = server_name {
        components.push_str(DESKTOP_TITLE_SEPARATOR);
        components.push_str(name);
    }

    if let Some(name) = document_name {
        components.push_str(DESKTOP_TITLE_SEPARATOR);
        components.push_str(name);
    }

    components.into()
}
