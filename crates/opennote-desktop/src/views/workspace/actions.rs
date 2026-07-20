use gpui::*;
use gpui_component::Root;

use crate::{
    globals::states::States,
    key_mappings::mappings::{
        CreateOneBlock, ToggleCommandBar, ToggleSearchBar, ToggleSettingsPanel, ToggleSidebar,
    },
};

use super::Workspace;

impl Workspace {
    /// Toggle the sidebar visibility and shift focus accordingly.
    pub fn toggle_sidebar(
        &mut self,
        _action: &ToggleSidebar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            this.toggle(cx);

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled() {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled() {
                let states: &States = cx.global();
                if let Some(tree_state) = this.get_tree_focus_handle(cx, &states.active_server) {
                    window.focus(&tree_state);
                }
            }
        });

        cx.notify();
    }

    /// Toggle the search bar and shift focus accordingly.
    pub fn toggle_search_bar(
        &mut self,
        _action: &ToggleSearchBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.search_bar.update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled {
                window.focus(&this.get_input_field_focus_handle(cx));
            }
        });

        cx.notify();
    }

    /// Toggle the command bar and shift focus accordingly.
    pub fn toggle_command_bar(
        &mut self,
        _action: &ToggleCommandBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_bar.update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled {
                window.focus(&this.get_input_field_focus_handle(cx));
            }
        });

        cx.notify();
    }

    /// Create a new block in the active server's tree.
    pub fn create_one_block(
        &mut self,
        _action: &CreateOneBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            let states: &States = cx.global();
            let tree_state = this.get_tree_state(&states.active_server);

            if let Some(tree_state) = tree_state {
                this.handle_block_creation(window, cx, tree_state);
            }
        })
    }

    /// Open the settings panel in a new window.
    pub fn toggle_settings_panel(
        &mut self,
        _action: &ToggleSettingsPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings_panel = self.settings_panel.clone();
        let _ = cx
            .open_window(WindowOptions::default(), |_this, cx| {
                cx.new(|cx| Root::new(settings_panel, window, cx))
            })
            .unwrap();
    }
}
