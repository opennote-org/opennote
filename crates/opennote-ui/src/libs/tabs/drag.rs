use gpui::{Context, ElementId, IntoElement, ParentElement, Render, SharedString, Styled, div};
use gpui_component::ActiveTheme;

// TODO:
// - create a single DraggedItem for dragging operations on the same layer
// - editor tabs can be dragged to split screens
// - sidebar tabs can be dragged to re-order
// - sidebar tabs can be dragged to open and split


#[derive(Debug, Clone)]
pub struct DraggedItem {
    pub element_id: ElementId,
    pub label: Option<SharedString>,
}

impl Render for DraggedItem {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let label = if let Some(label) = self.label.clone() {
            label
        } else {
            SharedString::new("")
        };

        div()
            .px_3()
            .py_1()
            .rounded_md()
            .shadow_md()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .opacity(0.85)
            .text_sm()
            .child(label)
    }
}
