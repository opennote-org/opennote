use std::ops::Range;

use gpui_kit::{App, FontWeight, HighlightStyle, SharedString, StyledText, component::ActiveTheme};

/// Highlight literal query terms without changing the displayed text or its UTF-8 byte offsets.
pub fn highlight_keyword_text(text: SharedString, query: &str, cx: &App) -> StyledText {
    // SQLite LIKE is case-insensitive for ASCII; ASCII folding preserves byte offsets.
    let folded_text = text.to_ascii_lowercase();

    let mut ranges: Vec<Range<usize>> = query
        .split_whitespace()
        .flat_map(|term| {
            folded_text
                .match_indices(&term.to_ascii_lowercase())
                .map(|(start, matched)| start..start + matched.len())
                .collect::<Vec<_>>()
        })
        .collect();

    ranges.sort_by_key(|range| (range.start, range.end));

    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if let Some(previous) = merged.last_mut() {
            if range.start <= previous.end {
                previous.end = previous.end.max(range.end);
                continue;
            }
        }
        merged.push(range);
    }

    let style = HighlightStyle {
        background_color: Some(cx.theme().yellow.opacity(0.25)),
        font_weight: Some(FontWeight::BOLD),
        ..Default::default()
    };

    StyledText::new(text).with_highlights(merged.into_iter().map(|range| (range, style)))
}
