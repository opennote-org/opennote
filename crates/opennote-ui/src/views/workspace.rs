use anyhow::Context as AnyhowContext;
use gpui::{Context, *};
use gpui_component::{
    Root, StyledExt, WindowExt,
    input::{InputEvent, InputState},
    notification::NotificationType,
};

use opennote_core_logics::note::read_blocks;
use opennote_data::database::enums::BlockQuery;

use crate::{
    globals::{
        bootstrap::GlobalApplicationBootStrap, helpers::get_language_profile, states::States,
    },
    key_mappings::mappings::{ToggleSearchBar, ToggleSidebar},
    widgets::{search_bar::SearchBar, sidebar::create_sidebar},
};

pub struct Workspace {
    focus_handle: FocusHandle,

    is_sidebar_toggled: bool,

    search_query: Entity<InputState>,
    search_query_text: SharedString,
    is_search_bar_toggled: bool,

    is_initialization_succeeded: bool,

    _subscriptions: Vec<Subscription>,
}

/// GPUI needs to have this trait implemented if it needs
/// to have action bindings
impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Result<Self> {
        let language_profile = get_language_profile(cx.global(), cx.global())
            .context("Getting language profile failed")?;

        let search_query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(&language_profile.search_bar_placeholder)
        });

        let mut _subscriptions = vec![];

        // Reserved for capturing search queries
        _subscriptions.push(cx.subscribe_in(&search_query, window, {
            let search_query = search_query.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = search_query.read(cx).value();
                    this.search_query_text = format!("{}", value).into();
                    cx.notify()
                }
                _ => {}
            }
        }));

        // // When the States changes, the workspace should refresh to reflect the change
        // _subscriptions.push(cx.observe_global::<States>(|this, cx| {
        //     cx.notify();
        // }));

        Ok(Self {
            focus_handle: cx.focus_handle(),
            search_query,
            search_query_text: "".into(),
            is_search_bar_toggled: false,
            is_sidebar_toggled: false,
            is_initialization_succeeded: false,
            _subscriptions,
        })
    }

    pub fn publish_initialization_successful_message(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_initialization_succeeded {
            window.push_notification(
                (
                    NotificationType::Success,
                    "Embedder model has been loaded successfully",
                ),
                cx,
            );
            self.is_initialization_succeeded = true;
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let search_bar = SearchBar::new(self.search_query.clone(), self.is_search_bar_toggled);
        let notification = Root::render_notification_layer(window, cx);

        self.publish_initialization_successful_message(window, cx);

        div()
            .key_context("workspace")
            .track_focus(&self.focus_handle) // GPUI needs this to get the focus of this workspace
            .v_flex()
            .h_full()
            .child(search_bar)
            .child(create_sidebar(self.is_sidebar_toggled, cx))
            .on_action(
                cx.listener(|workspace, _action: &ToggleSidebar, _window, cx| {
                    workspace.is_sidebar_toggled = !workspace.is_sidebar_toggled;
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|workspace, _action: &ToggleSearchBar, _window, cx| {
                    workspace.is_search_bar_toggled = !workspace.is_search_bar_toggled;
                    cx.notify();
                }),
            )
            .children(notification)
    }
}
