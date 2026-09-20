use gpui::{Entity, Global, WindowHandle};
use gpui_component::Root;

use crate::views::log::LogWindow;

/// Own the view even while its window is closed, keeping recent logs available
/// and sharing one receiver across all workspace windows.
pub struct GlobalLogWindowState {
    view: Entity<LogWindow>,
    window: Option<WindowHandle<Root>>,
}

impl Global for GlobalLogWindowState {}

impl GlobalLogWindowState {
    pub fn new(view: Entity<LogWindow>, window: Option<WindowHandle<Root>>) -> Self {
        Self { view, window }
    }

    pub fn take_handle(&mut self) -> Option<WindowHandle<Root>> {
        self.window.take()
    }

    pub fn update_handle(&mut self, handle: WindowHandle<Root>) {
        self.window = Some(handle);
    }

    pub fn get_view(&self) -> Entity<LogWindow> {
        self.view.clone()
    }
}
