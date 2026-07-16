use gpui::{
    AppContext, BorrowAppContext, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    WeakEntity, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    v_flex,
};

use opennote_core_logics::configurations::{ApplicationType, get_configuration_folder_path};
use opennote_models::{configurations::Configurations, traits::LoadFromAndSaveToFile};

use crate::{
    globals::{bootstrap::GlobalApplicationBootStrap, helpers::run_async_code, states::States},
    widgets::sidebar::OpenNoteSidebar,
};

pub struct SettingsPanel {
    /// The code editor entity.
    editor_state: Entity<InputState>,
    /// Feedback line shown below the toolbar.
    status_message: Option<String>,
    /// When `true`, the status message is rendered in an error colour.
    status_is_error: bool,
    /// This is used to refresh the sidebar after updated the config.
    sidebar: WeakEntity<OpenNoteSidebar>,
}

impl SettingsPanel {
    pub fn new(
        cx: &mut gpui::Context<Self>,
        window: &mut gpui::Window,
        sidebar: WeakEntity<OpenNoteSidebar>,
    ) -> Self {
        let config_json = {
            let bootstrap: &GlobalApplicationBootStrap = cx.global();
            let configs = run_async_code(async { bootstrap.0.configurations.lock().await.clone() });
            serde_json::to_string_pretty(&configs)
                .unwrap_or_else(|err| format!("// Failed to serialise config: {}\n{{}}", err))
        };

        let editor_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .multi_line(true)
                .line_number(true)
                .searchable(true)
                .soft_wrap(false)
                .default_value(config_json)
        });

        Self {
            editor_state,
            status_message: None,
            status_is_error: false,
            sidebar,
        }
    }

    /// Serialise the current `Configurations` to pretty-printed JSON.
    fn load_configs_to_json(cx: &mut gpui::Context<Self>) -> String {
        let bootstrap: &GlobalApplicationBootStrap = cx.global();
        let configs = run_async_code(async { bootstrap.0.configurations.lock().await.clone() });
        serde_json::to_string_pretty(&configs)
            .unwrap_or_else(|err| format!("// Failed to serialise config: {}\n{{}}", err))
    }

    /// Parse JSON into `Configurations`, validate, and pretty-print on success.
    fn parse_configs_json(json_text: &str) -> Result<(Configurations, String), String> {
        let configs: Configurations =
            serde_json::from_str(json_text).map_err(|err| format!("Invalid JSON: {}", err))?;
        configs
            .validate()
            .map_err(|err| format!("Validation error: {}", err))?;
        let pretty = serde_json::to_string_pretty(&configs)
            .map_err(|err| format!("Serialisation error: {}", err))?;
        Ok((configs, pretty))
    }

    /// Replace the entire editor content with `text`.
    fn set_editor_text(
        editor: &Entity<InputState>,
        replacement_text: &str,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let replacement_text = replacement_text.to_string();
        editor.update(cx, |state, cx| {
            state.set_value(replacement_text, window, cx);
        });
    }

    /// Save: parse JSON → validate → persist to disk → reload in-memory config.
    fn save_configurations(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let current_text = self.editor_state.read(cx).value();
        let config_path = get_configuration_folder_path(ApplicationType::Desktop);

        match Self::parse_configs_json(&current_text) {
            Ok((parsed_configs, pretty_json)) => {
                let servers = parsed_configs.user.remote_servers.clone();

                cx.update_global::<GlobalApplicationBootStrap, ()>(move |bootstrap, _cx| {
                    run_async_code(async {
                        let mut configs = bootstrap.0.configurations.lock().await;
                        *configs = parsed_configs;
                        configs
                            .save_to_file(&config_path)
                            .expect("Failed to save configurations");
                    });
                });

                cx.update_global::<States, ()>(move |this, _cx| {
                    this.update_servers(servers);
                });

                Self::set_editor_text(&self.editor_state, &pretty_json, window, cx);

                self.status_message = Some("✓ Configurations saved successfully.".into());
                self.status_is_error = false;
            }
            Err(err) => {
                self.status_message = Some(format!("✗ {}", err));
                self.status_is_error = true;
            }
        }

        cx.notify();
    }

    /// Reload: re-read configs from the bootstrap and refresh the editor.
    fn reload_configurations(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let pretty_json = Self::load_configs_to_json(cx);
        Self::set_editor_text(&self.editor_state, &pretty_json, window, cx);

        self.status_message = Some("↻ Configurations reloaded from disk.".into());
        self.status_is_error = false;
        cx.notify();
    }
}

impl Render for SettingsPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let editor = &self.editor_state;

        // Compute the status colour.
        let status_color = if self.status_is_error {
            cx.theme().danger
        } else {
            cx.theme().success
        };

        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    // Buttons
                    .child(
                        Button::new("save-configs")
                            .primary()
                            .small()
                            .label("Save")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_configurations(window, cx);
                                // Reload the sidebar for updating the servers
                                let _ = this.sidebar.update(cx, |_this, cx| {
                                    cx.update_global::<States, ()>(|this, cx| {
                                        this.refresh_blocks_list(cx)
                                    });
                                    cx.notify();
                                });
                            })),
                    )
                    .child(
                        Button::new("reload-configs")
                            .outline()
                            .small()
                            .label("Reload")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reload_configurations(window, cx);
                            })),
                    )
                    // Spacer pushes the status message to the right.
                    .child(gpui::div().flex_1())
                    // Status message.
                    .when_some(self.status_message.as_ref(), |this, msg| {
                        this.child(
                            gpui::div()
                                .text_sm()
                                .text_color(status_color)
                                .child(msg.clone()),
                        )
                    }),
            )
            // JSON editor
            .child(
                gpui::div()
                    .flex_1()
                    .w_full()
                    .child(Input::new(editor).h_full()),
            )
            .into_any_element()
    }
}
