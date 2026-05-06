use std::io::Read;

use anyhow::{Result, anyhow};
use chunk::chunk;
use serde_json::Value;
use uuid::Uuid;

use opennote_models::{content_type::ContentType, payload::Payload};

pub struct PayloadContentParameters {
    pub title: Option<String>,
    pub markdown: Option<String>,
    pub image: Option<Vec<u8>>,
    pub json: Option<Value>,
}

impl Default for PayloadContentParameters {
    fn default() -> Self {
        Self {
            title: None,
            markdown: None,
            image: None,
            json: None,
        }
    }
}

/// Create payloads without vectors
/// If one of the payloads failed, it will fail all
pub fn build_payloads(
    creation_parameters: impl Iterator<Item = (Uuid, PayloadContentParameters)>,
) -> Result<Vec<Payload>> {
    let mut payloads = Vec::new();

    for (block_id, parameters) in creation_parameters {
        payloads.push(build_payload(block_id, parameters)?);
    }

    Ok(payloads)
}

/// Create a payload without vectors
pub fn build_payload(block_id: Uuid, parameters: PayloadContentParameters) -> Result<Payload> {
    if let Some(_json) = parameters.json {
        todo!()
    }

    if let Some(_image) = parameters.image {
        todo!()
    }

    if parameters.title.is_some() && parameters.markdown.is_some() {
        return Err(anyhow!(
            "You should supply either the title or the markdown, but not both"
        ));
    }

    if let Some(title) = parameters.title {
        return Ok(Payload::new(
            block_id,
            ContentType::Title,
            title,
            vec![],
            vec![],
        ));
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
pub fn convert_string_to_payloads(
    block_id: Uuid,
    text_chunk_size: Option<usize>,
    string: String,
) -> Result<Vec<Payload>> {
    let mut payloads: Vec<Payload> = Vec::new();

    let mut chunker = chunk(string.as_bytes())
        .consecutive()
        .patterns(&["。", "！", "，", "？"])
        .delimiters("\n.?!".as_bytes());

    // Limit chunk size when specified
    if let Some(text_chunk_size) = text_chunk_size {
        chunker = chunker.size(text_chunk_size);
    }

    let raw_chunks: Vec<&[u8]> = chunker.collect();

    for (index, mut chunk) in raw_chunks.into_iter().enumerate() {
        let mut bytes = Vec::new();
        match chunk.read_to_end(&mut bytes) {
            Ok(_) => {
                // The first chunk is always the title
                if index == 0 {
                    payloads.push(build_payload(
                        block_id,
                        PayloadContentParameters {
                            title: Some(String::from_utf8_lossy(&bytes).to_string()),
                            ..Default::default()
                        },
                    )?);
                    continue;
                }

                payloads.push(build_payload(
                    block_id,
                    PayloadContentParameters {
                        markdown: Some(String::from_utf8_lossy(&bytes).to_string()),
                        ..Default::default()
                    },
                )?);
            }
            Err(error) => return Err(error.into()),
        }
    }

    Ok(payloads)
}
