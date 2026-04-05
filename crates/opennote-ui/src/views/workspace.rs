use gpui::*;
use gpui_component::{
    StyledExt,
    input::{InputEvent, InputState},
};

use crate::{
    globals::{assets::AssetsCollection, bootstrap::UIApplicationBootStrap},
    key_mappings::mappings::{ToggleSearchBar, ToggleSidebar},
    widgets::{search_bar::SearchBar, sidebar::Sidebar},
};

pub struct Workspace {
    focus_handle: FocusHandle,

    is_sidebar_toggled: bool,

    search_query: Entity<InputState>,
    search_query_text: SharedString,
    is_search_bar_toggled: bool,

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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language_profile = {
            let services_and_resources: &UIApplicationBootStrap = cx.global();
            let assets_collection: &AssetsCollection = cx.global();

            let language = services_and_resources.0.configurations.user.language.to_string();
            assets_collection
                .language_profiles
                .get(&language)
                .unwrap()
                .to_owned()
        };

        let search_query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(&language_profile.search_bar_placeholder)
        });

        let _subscriptions = vec![cx.subscribe_in(&search_query, window, {
            let search_query = search_query.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = search_query.read(cx).value();
                    this.search_query_text = format!("{}", value).into();
                    cx.notify()
                }
                _ => {}
            }
        })];

        Self {
            focus_handle: cx.focus_handle(),
            search_query,
            search_query_text: "".into(),
            is_search_bar_toggled: false,
            is_sidebar_toggled: false,
            _subscriptions,
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = Sidebar::new(self.is_sidebar_toggled);

        let search_bar = SearchBar::new(self.search_query.clone(), self.is_search_bar_toggled);

        div()
            .v_flex()
            .key_context("workspace")
            .h_full()
            .track_focus(&self.focus_handle) // GPUI needs this to get the focus of this workspace
            .child(search_bar)
            .child(sidebar)
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
    }
}
