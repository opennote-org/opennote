use gpui::{Action, Context, IntoElement, Render, Window};

pub trait Widget: Sized {
    /// Toggle this widget on and off
    fn toggle(&mut self, action: &dyn Action, window: &mut Window, cx: &mut Context<impl Render>);

    /// Create this widget in the render tree
    fn create(&self, window: &mut Window, cx: &mut Context<impl Render>) -> impl IntoElement;
}
