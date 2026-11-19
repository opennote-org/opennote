use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ContentType {
    Title,    // Legacy type
    Markdown, // Markdown contents that are not specified here
    Image,    // ![alt text](image_url)
    Heading,
    Link,
    ListItem,
    CodeBlock,
    HtmlBlock,
    BlockQuote,
}
