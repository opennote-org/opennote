use std::ops::Range;

use gpui::*;

pub struct LogEntry {
    text: String,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
}

impl LogEntry {
    pub fn get_text(&self) -> SharedString {
        self.text.clone().into()
    }

    pub fn get_highlights(&self) -> &Vec<(Range<usize>, HighlightStyle)> {
        &self.highlights
    }

    /// Interpret the SGR styles emitted by tracing's default ANSI formatter.
    pub fn from_ansi(input: &str) -> Self {
        let mut text = String::new();
        let mut highlights = Vec::new();
        let mut style = HighlightStyle::default();
        let mut remaining = input.strip_suffix('\n').unwrap_or(input);

        while !remaining.is_empty() {
            if let Some(after_escape) = remaining.strip_prefix("\x1b[") {
                if let Some(end) = after_escape.find('m') {
                    for code in after_escape[..end].split(';') {
                        match code.parse::<u8>().unwrap_or(0) {
                            0 => style = HighlightStyle::default(),
                            1 => style.font_weight = Some(FontWeight::BOLD),
                            2 => style.color = Some(rgb(0xa0a6ad).into()),
                            22 => {
                                style.font_weight = None;
                                style.color = None;
                            }
                            30..=37 => {
                                let palette = [
                                    0x000000, 0xff5555, 0x28cd41, 0xe5c07b, 0x4aa5ff, 0xc678dd,
                                    0x56b6c2, 0xe6edf3,
                                ];
                                let code = code.parse::<usize>().unwrap();
                                style.color = Some(rgb(palette[code - 30]).into());
                            }
                            39 => style.color = None,
                            _ => {}
                        }
                    }
                    remaining = &after_escape[end + 1..];
                    continue;
                }
            }

            let end = remaining
                .find("\x1b[")
                .filter(|end| *end > 0)
                .unwrap_or(remaining.len());
            let start = text.len();

            text.push_str(&remaining[..end]);
            highlights.push((start..text.len(), style.clone()));

            remaining = &remaining[end..];
        }

        Self { text, highlights }
    }
}
