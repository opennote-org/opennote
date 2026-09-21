use gpui_kit::{Edges, Hsla, Pixels, px};

#[allow(dead_code)]
pub struct TabStyle {
    pub borders: Edges<Pixels>,
    pub border_color: Hsla,
    pub bg: Hsla,
    pub fg: Hsla,
    pub radius: Pixels,
    pub shadow: bool,
    pub inner_bg: Hsla,
    pub inner_radius: Pixels,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: gpui_kit::transparent_white(),
            bg: gpui_kit::transparent_white(),
            fg: gpui_kit::transparent_white(),
            radius: px(0.),
            shadow: false,
            inner_bg: gpui_kit::transparent_white(),
            inner_radius: px(0.),
        }
    }
}
