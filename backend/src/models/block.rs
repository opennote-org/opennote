use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    /// Parent ID of this block. Root blocks don't have parent ids.
    pub parent_id: Option<String>,
    /// Reserved for soft deletion
    pub is_deleted: bool,
    /// Actual data contained in this block
    pub payloads: Vec<Payload>,
}

/// Next: do we store dynamic data? like hashmap?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub id: String,
    /// The position (row) of the payload on a block.
    pub order_row: i64,
    /// The position (column) of the payload on a block.
    pub order_column: i64,
    /// When this payload is created
    pub created_at: i64,
    /// Last time this payload is modified
    pub last_modified: i64,
    /// Content type presented in which style. For example, text can be P1 or so.
    pub content_type: ContentType,
    /// Texts stored in payload
    pub texts: String,
    /// Bytes stored in payload. Typically, we modalities other than texts, like images
    pub bytes: Vec<u8>,
    /// Vector representation of the stored texts or bytes
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContentType {
    Title,             // Only the first payload of a block has title
    HeadingLevel1,     // # Heading
    HeadingLevel2,     // ## Heading
    HeadingLevel3,     // ### Heading
    HeadingLevel4,     // #### Heading
    HeadingLevel5,     // ##### Heading
    HeadingLevel6,     // ###### Heading
    Bold,              // **bold text**
    Italic,            // *italic text*
    BoldAndItalic,     // ***bold and italic text***
    BlockQuotes,       // > block quote
    UnorderedListItem, // - item
    OrderedListItem,   // 1. item
    Code,              // `inline code`
    CodeBlock,         // ```code block```
    Horizontal,        // ---
    Link,              // [link text](url)
    URLAndEmail,       // <https://example.com>
    Reference,         // [link text][reference]
    Source,            // [reference]: https://example.com
    Image,             // ![alt text](image_url)
    Body,
}
