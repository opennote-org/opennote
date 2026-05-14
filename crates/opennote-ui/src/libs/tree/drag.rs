use gpui::{Context, IntoElement, ParentElement, Render, SharedString, Styled, div};
use gpui_component::ActiveTheme;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DraggedBlocks {
    pub block_id: Uuid,
    pub label: SharedString,
    pub current_selections: Vec<Uuid>,
}

impl Render for DraggedBlocks {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .shadow_md()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .opacity(0.85)
            .text_sm()
            .child(format!("{} items", self.current_selections.len()))
    }
}
