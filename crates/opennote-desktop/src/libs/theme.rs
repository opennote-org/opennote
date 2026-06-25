use gpui::*;
use gpui_component::*;

/// Adapt the app theme to the system.
///
/// The internal behavior varies by platform:
/// On macOS, it will make a call to the NS lib.
/// On Windows, it will check against the registry for the corresponding value.
/// On Linux, it will need to connect to XDG Desktop Portal.
pub fn adapt_theme_to_system(cx: &mut App) {
    let theme_registry = ThemeRegistry::global(cx);
    let theme = match dark_light::detect().unwrap() {
        dark_light::Mode::Dark => theme_registry.default_dark_theme().clone(),
        dark_light::Mode::Light => theme_registry.default_light_theme().clone(),
        dark_light::Mode::Unspecified => theme_registry.default_light_theme().clone(),
    };

    Theme::global_mut(cx).apply_config(&theme);
}
