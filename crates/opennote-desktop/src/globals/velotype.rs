use gpui::App;

use opennote_core_logics::configurations::{ApplicationType, get_configuration_folder_path};
use opennote_velotype::{
    app_menu,
    components::init_with_keybindings,
    config::preferences::{EditorSettings, load_or_create_app_preferences_with_path},
    i18n::I18nManager,
    net::install_http_client,
    theme::ThemeManager,
};

pub fn init_velotype(cx: &mut App) {
    let preferences = load_or_create_app_preferences_with_path(get_configuration_folder_path(
        ApplicationType::Desktop,
    ))
    .unwrap_or_else(|err| {
        eprintln!("failed to initialize app preferences: {err}");
        Default::default()
    });
    I18nManager::init_with_language_id(cx, &preferences.default_language_id);
    ThemeManager::init_with_theme_id(cx, &preferences.default_theme_id);
    EditorSettings::init(cx, preferences.show_table_headers);
    install_http_client(cx);
    init_with_keybindings(cx, &preferences.keybindings);
    app_menu::init(cx);
}
