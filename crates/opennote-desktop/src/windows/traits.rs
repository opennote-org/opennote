use anyhow::{Error, Result};
use gpui_kit::{App, WindowHandle};

/// A collection of operations that are shared between windows.
pub trait WindowObject<T> {
    fn open(cx: &mut App) -> Result<WindowHandle<T>, Error>;
}
