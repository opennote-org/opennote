use gpui::{
    AnyElement, App, EdgesRefinement, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, Window, prelude::FluentBuilder, px,
};

use crate::{
    ActiveTheme, Side, StyledExt, h_flex, libs::tree::Tree, scroll::ScrollableElement, v_flex,
};

const DEFAULT_WIDTH: Pixels = px(255.);

/// A Sidebar element that can contain trees
#[derive(IntoElement)]
pub struct TreeViewSidebar {
    style: StyleRefinement,
    content: Vec<Tree>,
    /// header view
    header: Option<AnyElement>,
    /// footer view
    footer: Option<AnyElement>,
    /// The side of the sidebar
    side: Side,
}

impl TreeViewSidebar {
    /// Create a new Sidebar on the given [`Side`].
    pub fn new(side: Side) -> Self {
        Self {
            style: StyleRefinement::default(),
            content: vec![],
            header: None,
            footer: None,
            side,
        }
    }

    /// Create a new Sidebar on the left side.
    pub fn left() -> Self {
        Self::new(Side::Left)
    }

    /// Create a new Sidebar on the right side.
    pub fn right() -> Self {
        Self::new(Side::Right)
    }

    /// Set the header of the sidebar.
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Set the footer of the sidebar.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Add a child element to the sidebar
    pub fn child(mut self, child: Tree) -> Self {
        self.content.push(child);
        self
    }

    /// Add multiple children to the sidebar
    pub fn children(mut self, children: impl IntoIterator<Item = Tree>) -> Self {
        self.content.extend(children);
        self
    }
}

impl Styled for TreeViewSidebar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TreeViewSidebar {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.style.padding = EdgesRefinement::default();

        v_flex()
            .id("sidebar")
            .w(DEFAULT_WIDTH)
            .flex_shrink_0()
            .h_full()
            .overflow_hidden()
            .relative()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .border_color(cx.theme().sidebar_border)
            .map(|this| match self.side {
                Side::Left => this.border_r_1(),
                Side::Right => this.border_l_1(),
            })
            .refine_style(&self.style)
            .when_some(self.header.take(), |this, header| {
                this.child(h_flex().id("header").pt_3().px_3().gap_2().child(header))
            })
            .child(
                v_flex().id("content").flex_1().min_h_0().child(
                    v_flex()
                        .id("inner")
                        .p_3()
                        .children(
                            // self.content
                            //     .into_iter()
                            //     .enumerate()
                            //     .map(|(ix, c)| {
                            //         dbg!("Rendering", ix);
                            //         div().id(ix).mt_3().child(c)
                            //     }),
                            self.content,
                        )
                        .overflow_y_scrollbar(),
                ),
            )
            .when_some(self.footer.take(), |this, footer| {
                this.child(h_flex().id("footer").pb_3().px_3().gap_2().child(footer))
            })
    }
}
