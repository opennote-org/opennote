use anyhow::{Result, anyhow};
use pulldown_cmark::{Event, Parser, TextMergeWithOffset};
use serde_json::Value;
use uuid::Uuid;

use opennote_models::{content_type::ContentType, payload::Payload};

pub struct PayloadContentParameters {
    pub markdown: Option<String>,
    pub image: Option<Vec<u8>>,
    pub json: Option<Value>,
}

impl Default for PayloadContentParameters {
    fn default() -> Self {
        Self {
            markdown: None,
            image: None,
            json: None,
        }
    }
}

/// Create a payload without vectors
pub fn build_payload(block_id: Uuid, parameters: PayloadContentParameters) -> Result<Payload> {
    if let Some(_json) = parameters.json {
        todo!()
    }

    if let Some(_image) = parameters.image {
        todo!()
    }

    if let Some(markdown) = parameters.markdown {
        return Ok(Payload::new(
            block_id,
            ContentType::Markdown,
            markdown,
            vec![],
            vec![],
        ));
    }

    Err(anyhow!(
        "No payload creation case matches the input. Please check the payload creation inputs"
    ))
}

/// Convert a string to payloads but without vectors.
///
/// TODO: need to consider multi-modal support
pub fn build_payloads(
    block_id: Uuid,
    string: String,
    default_title: Option<String>, // Use the default title as the title if supplied
) -> Result<Vec<Payload>> {
    let mut payloads: Vec<Payload> = Vec::new();

    if let Some(mut title) = default_title.clone() {
        title.push('\n');

        payloads.push(build_payload(
            block_id,
            PayloadContentParameters {
                markdown: Some(title),
                ..Default::default()
            },
        )?);
    }

    let iterator = TextMergeWithOffset::new(Parser::new(&string).into_offset_iter());

    let mut depth: usize = 0;
    let mut cursor: usize = 0;

    for (index, (event, range)) in iterator.enumerate() {
        match event {
            Event::Start(_tag) => {
                if depth == 0 && range.start > cursor {
                    payloads.push(build_payload(
                        block_id,
                        PayloadContentParameters {
                            markdown: Some(string[cursor..range.start].to_string()),
                            ..Default::default()
                        },
                    )?);

                    cursor = range.start;
                }

                depth += 1;
            }
            Event::End(_tag) => depth -= 1,
            _ => continue,
        }
    }

    payloads.push(build_payload(
        block_id,
        PayloadContentParameters {
            markdown: Some(string[cursor..].to_string()),
            ..Default::default()
        },
    )?);

    Ok(payloads)
}
