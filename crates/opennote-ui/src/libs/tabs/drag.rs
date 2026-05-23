use gpui::{Context, IntoElement, ParentElement, Render, SharedString, Styled, WeakEntity, div};
use gpui_component::ActiveTheme;
use uuid::Uuid;

use crate::widgets::pane::pane::Pane;

// TODO:
// - create a single DraggedItem for dragging operations on the same layer
// - editor tabs can be dragged to split screens
// - sidebar tabs can be dragged to re-order
// - sidebar tabs can be dragged to open and split

#[derive(Debug, Clone)]
pub struct DraggedItem {
    /// A semantic label of this item
    pub label: Option<SharedString>,

    /// The pane who owns this item.
    /// None means no pane owns this item.
    pub owner_pane: Option<WeakEntity<Pane>>,

    /// Owner pane's uuid, for convenience access.
    pub owner_pane_id: Option<Uuid>,

    /// The block id this item may carry.
    /// None means no block id is carried.
    pub block_id: Option<Uuid>,
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
