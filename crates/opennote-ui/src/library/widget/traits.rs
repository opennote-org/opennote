use gpui::{Action, Context, IntoElement, Render, Window};

pub trait Widget: Sized {
    /// Toggle this widget on and off
    fn toggle(&mut self, action: &dyn Action, window: &mut Window, cx: &mut Context<impl Render>);
}
