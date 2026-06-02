use anyhow::{Context as AnyhowContext, Result};
use gpui::{
    AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, ParentElement, Render,
    Styled, Subscription, div, prelude::FluentBuilder,
};
use gpui_component::{
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};

use crate::globals::helpers::get_language_profile;

/// Select commands to execute
pub struct CommandBar {
    pub is_toggled: bool,
    pub input_state: Entity<InputState>,

    pub focus_handle: FocusHandle,
    pub _subscriptions: Vec<Subscription>,
}

/// TODO:
/// - Implement commands (toggle sidebar and settings panel) for CommandBar
/// - Implement settings panel
/// - Create default background for editor
/// - Should focus on the input field when the input box shows up
impl CommandBar {
    pub fn new(cx: &mut Context<Self>, window: &mut gpui::Window) -> Result<Self> {
        let mut _subscriptions = Vec::new();

        let language_profile = get_language_profile(cx.global(), cx.global())
            .context("Getting language profile failed")?;

        let input_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(&language_profile.command_bar_placeholder)
        });

        _subscriptions.push(cx.subscribe_in(&input_state, window, {
            move |_this, input_state, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    dbg!("hello");
                    let value = input_state.read(cx).value();
                    dbg!(&value);
                    cx.notify()
                }
                _ => {}
            }
        }));

        Ok(Self {
            is_toggled: false,
            input_state,
            focus_handle: cx.focus_handle(),
            _subscriptions,
        })
    }
}

impl Focusable for CommandBar {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandBar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .track_focus(&self.focus_handle(cx))
            .absolute()
            .size_full()
            .when(self.is_toggled, |this| this.visible())
            .when(!self.is_toggled, |this| this.invisible())
            .child(
                v_flex().top_20().items_center().child(
                    h_flex()
                        .w_128() // Apply a default width of the search bar
                        .child(Input::new(&self.input_state)),
                ),
            )
    }
}
