use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ContentType {
    Title,     // Only the first payload of a block has title
    Image,     // ![alt text](image_url)
    JSONValue, // A non-typed json value
    Markdown,  // Markdown contents
}
