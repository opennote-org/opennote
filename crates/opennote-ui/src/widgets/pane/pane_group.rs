//! TODO:
//! - Might need to introduce `basis` to renders to avoid fast drag issues

use gpui::{
    AnyElement, App, Axis, Bounds, Element, Entity, IntoElement, ParentElement, Pixels, Render,
    Styled, Window, div, point, relative, size,
};
use gpui_component::{h_flex, v_flex};
use serde::Deserialize;

use crate::widgets::pane::pane::Pane;

pub const HANDLE_HITBOX_SIZE: f32 = 4.0;

/// One or many panes, arranged in a horizontal or vertical axis due to a split.
/// Panes have all their tabs and capabilities preserved, and can be split again or resized.
/// Single-pane group is a regular pane.
#[derive(Clone)]
pub struct PaneGroup {
    pub root: Member,
    pub is_center: bool,
}

impl PaneGroup {
    pub fn new(pane: Entity<Pane>) -> Self {
        Self {
            root: Member::Pane(pane),
            is_center: false,
        }
    }

    /// Only return an Entity<Pane> when the member is a pane.
    /// Otherwise, it will return a None.
    fn get_pane_from_member(pane_member: &Member) -> Option<Entity<Pane>> {
        match pane_member {
            Member::Axis(_) => None,
            Member::Pane(pane) => Some(pane.clone()),
        }
    }

    /// Return a pane for refocus after removal
    fn remove_pane(member: &mut Member, pane: &Entity<Pane>) -> Option<Entity<Pane>> {
        // Recursively find the pane to remove
        match member {
            Member::Axis(axis) => {
                if let Some(pane_left) = axis.remove_pane(pane) {
                    *member = pane_left;
                    return Self::get_pane_from_member(member);
                }
            }
            Member::Pane(member_pane) => {
                return Some(member_pane.clone());
            }
        };

        None
    }

    /// It will return a pane for refocus
    pub fn remove_panes(&mut self, pane: &Entity<Pane>) -> Option<Entity<Pane>> {
        Self::remove_pane(&mut self.root, pane)
    }

    /// It will return the a pane for refocus
    pub fn split(
        &mut self,
        old_pane: &Entity<Pane>,
        new_pane: &Entity<Pane>,
        direction: SplitDirection,
        old_pane_has_opened_blocks: bool,
    ) -> Option<Entity<Pane>> {
        match &mut self.root {
            Member::Pane(_pane) => {
                self.root = Member::new_axis(old_pane.clone(), new_pane.clone(), direction);
            }
            Member::Axis(axis) => {
                let _ = axis.split(old_pane, new_pane, direction);
            }
        };

        // Remove the old pane if it has no tabs left
        if !old_pane_has_opened_blocks {
            return Self::remove_pane(&mut self.root, old_pane);
        }

        None
    }
}

#[derive(Debug, Clone)]
pub enum Member {
    Axis(PaneAxis),
    Pane(Entity<Pane>),
}

impl Member {
    fn new_axis(old_pane: Entity<Pane>, new_pane: Entity<Pane>, direction: SplitDirection) -> Self {
        use Axis::*;
        use SplitDirection::*;

        let axis = match direction {
            Up | Down => Vertical,
            Left | Right => Horizontal,
        };

        let members = match direction {
            Up | Left => vec![Member::Pane(new_pane), Member::Pane(old_pane)],
            Down | Right => vec![Member::Pane(old_pane), Member::Pane(new_pane)],
        };

        Member::Axis(PaneAxis::new(axis, members))
    }

    pub fn render(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Member::Axis(axis) => axis.render(window, cx),
            Member::Pane(pane) => pane.clone().into_any_element(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaneAxis {
    pub axis: Axis,
    pub members: Vec<Member>,
}

impl PaneAxis {
    pub fn new(axis: Axis, members: Vec<Member>) -> Self {
        Self { axis, members }
    }

    pub fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        // What is the use of axis variable of PaneAxis?
        // Determine the direction of the Panes in this PaneAxis
        let mut div = match self.axis {
            Axis::Horizontal => h_flex().size_full(),
            Axis::Vertical => v_flex().size_full(),
        };

        let num_members = self.members.len();

        for member in self.members.iter() {
            match member {
                // But how we are going to display the PaneAxis?
                // Recurse into it
                Member::Axis(axis) => {
                    div = div
                        .child(axis.render(window, cx))
                        .flex_basis(relative(1.0 / num_members as f32)) // Make each pane displayed in equal ratio
                }
                Member::Pane(pane) => {
                    div = div
                        // .flex_shrink()
                        // .min_w_0()
                        .child(pane.clone().into_any_element())
                        .flex_basis(relative(1.0 / num_members as f32)) // Same as the one above
                }
            }
        }

        div.into_any()
    }

    fn split(
        &mut self,
        old_pane: &Entity<Pane>,
        new_pane: &Entity<Pane>,
        direction: SplitDirection,
    ) -> bool {
        for (mut idx, member) in self.members.iter_mut().enumerate() {
            match member {
                Member::Axis(axis) => {
                    if axis.split(old_pane, new_pane, direction) {
                        return true;
                    }
                }
                Member::Pane(pane) => {
                    if pane == old_pane {
                        if direction.axis() == self.axis {
                            if direction.increasing() {
                                idx += 1;
                            }
                            self.insert_pane(idx, new_pane);
                        } else {
                            *member =
                                Member::new_axis(old_pane.clone(), new_pane.clone(), direction);
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    fn insert_pane(&mut self, idx: usize, new_pane: &Entity<Pane>) {
        self.members.insert(idx, Member::Pane(new_pane.clone()));
        // *self.flexes.lock() = vec![1.; self.members.len()];
    }

    /// Remove the given pane.
    /// Return the member if it only has 1 pane left after removal.
    fn remove_pane(&mut self, pane: &Entity<Pane>) -> Option<Member> {
        let mut pane_to_remove = None;
        for (index, member) in self.members.iter_mut().enumerate() {
            match member {
                Member::Axis(axis) => {
                    if let Some(pane) = axis.remove_pane(pane) {
                        *member = pane;
                    }
                }
                Member::Pane(member_pane) => {
                    if member_pane == pane {
                        pane_to_remove = Some(index);
                    }
                }
            }
        }

        if let Some(index) = pane_to_remove {
            self.members.remove(index);
        }

        if self.members.len() == 1 {
            return self.members.pop();
        }

        None
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for SplitDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplitDirection::Up => write!(f, "up"),
            SplitDirection::Down => write!(f, "down"),
            SplitDirection::Left => write!(f, "left"),
            SplitDirection::Right => write!(f, "right"),
        }
    }
}

impl SplitDirection {
    pub fn all() -> [Self; 4] {
        [Self::Up, Self::Down, Self::Left, Self::Right]
    }

    pub fn edge(&self, rect: Bounds<Pixels>) -> Pixels {
        match self {
            Self::Up => rect.origin.y,
            Self::Down => rect.bottom_left().y,
            Self::Left => rect.bottom_left().x,
            Self::Right => rect.bottom_right().x,
        }
    }

    pub fn along_edge(&self, bounds: Bounds<Pixels>, length: Pixels) -> Bounds<Pixels> {
        match self {
            Self::Up => Bounds {
                origin: bounds.origin,
                size: size(bounds.size.width, length),
            },
            Self::Down => Bounds {
                origin: point(bounds.bottom_left().x, bounds.bottom_left().y - length),
                size: size(bounds.size.width, length),
            },
            Self::Left => Bounds {
                origin: bounds.origin,
                size: size(length, bounds.size.height),
            },
            Self::Right => Bounds {
                origin: point(bounds.bottom_right().x - length, bounds.bottom_left().y),
                size: size(length, bounds.size.height),
            },
        }
    }

    pub fn axis(&self) -> Axis {
        match self {
            Self::Up | Self::Down => Axis::Vertical,
            Self::Left | Self::Right => Axis::Horizontal,
        }
    }

    pub fn increasing(&self) -> bool {
        match self {
            Self::Left | Self::Up => false,
            Self::Down | Self::Right => true,
        }
    }

    pub fn opposite(&self) -> SplitDirection {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl Render for PaneGroup {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .flex_col()
            .size_full()
            .child(self.root.render(window, cx)) // We need flex_1 to let the editor to take up the whole space after sidebar disappeared
    }
}
