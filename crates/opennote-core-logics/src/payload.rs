use anyhow::Result;
use pulldown_cmark::{Event, Parser, Tag, TextMergeWithOffset};
use uuid::Uuid;

use opennote_models::{content_type::ContentType, payload::Payload};

/// Convert a string to payloads but without vectors.
pub fn build_payloads(
    block_id: Uuid,
    string: String,
    default_title: Option<String>, // Use the default title as the title if supplied
) -> Result<Vec<Payload>> {
    let mut payloads: Vec<Payload> = Vec::new();

    if let Some(mut title) = default_title.clone() {
        title.push('\n');

        payloads.push(Payload::new(
            block_id,
            ContentType::Markdown,
            title,
            Vec::new(),
            Vec::new(),
        ));
    }

    let iterator = TextMergeWithOffset::new(Parser::new(&string).into_offset_iter());

    let mut cursor: usize = 0;

    for (_index, (event, range)) in iterator.enumerate() {
        if cursor > range.start {
            continue;
        }

        let content_type = match event {
            Event::Start(tag) => match tag {
                Tag::Heading { .. } => ContentType::Heading,
                Tag::Image { .. } => ContentType::Image,
                Tag::Link { .. } => ContentType::Link,
                Tag::Item => ContentType::ListItem,
                Tag::CodeBlock(_) => ContentType::CodeBlock,
                Tag::HtmlBlock => ContentType::HtmlBlock,
                Tag::BlockQuote(_) => ContentType::BlockQuote,
                _ => continue,
            },
            Event::End(_tag) => continue,
            _ => ContentType::Markdown,
        };

        payloads.push(Payload::new(
            block_id,
            content_type,
            string[cursor..range.end].to_string(),
            Vec::new(),
            Vec::new(),
        ));

        cursor = range.end;
    }

    payloads.push(Payload::new(
        block_id,
        ContentType::Markdown,
        string[cursor..].to_string(),
        Vec::new(),
        Vec::new(),
    ));

    Ok(payloads)
}
