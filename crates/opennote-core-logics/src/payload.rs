use anyhow::{Result, anyhow};
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
pub fn create_payloads(
    creation_parameters: impl Iterator<Item = (Uuid, PayloadContentParameters)>,
) -> Result<Vec<Payload>> {
    let mut payloads = Vec::new();

    for (block_id, parameters) in creation_parameters {
        payloads.push(create_payload(block_id, parameters)?);
    }

    Ok(payloads)
}

/// Create a payload without vectors
pub fn create_payload(block_id: Uuid, parameters: PayloadContentParameters) -> Result<Payload> {
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
