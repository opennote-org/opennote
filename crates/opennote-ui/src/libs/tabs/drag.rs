use gpui::{Context, IntoElement, ParentElement, Render, SharedString, Styled, WeakEntity, div};
use gpui_component::ActiveTheme;
use uuid::Uuid;

use crate::widgets::pane::pane::Pane;

// TODO:
// - sidebar tabs can be dragged to re-order
// - sidebar tabs can be dragged to open and split

/// When rendering, if label is available, num selections will be ignored.
#[derive(Debug, Clone)]
pub struct DraggedItem {
    /// The block id this item may carry.
    /// None means no block id is carried.
    pub block_id: Option<Uuid>,

    /// Selected blocks
    pub selections: Vec<Uuid>,

    /// A semantic label of this item
    pub label: Option<SharedString>,

    /// The pane who owns this item.
    /// None means no pane owns this item.
    pub owner_pane: Option<WeakEntity<Pane>>,

    /// Owner pane's uuid, for convenience access.
    pub owner_pane_id: Option<Uuid>,
}

impl Default for DraggedItem {
    fn default() -> Self {
        Self {
            block_id: None,
            selections: Vec::new(),
            label: None,
            owner_pane: None,
            owner_pane_id: None,
        }
    }
}

impl Render for DraggedItem {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let mut div = div()
            .px_3()
            .py_1()
            .rounded_md()
            .shadow_md()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .opacity(0.85)
            .text_sm();

        if let Some(label) = self.label.clone() {
            div = div.child(label);
        }

        // This means it is not a single selection.
        // Then we will display the items in multi-selection style.
        if self.selections.len() > 1 {
            div = div.child(format!("and {} other items", self.selections.len() - 1));
        }

        div
    }
}
