use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    /// Parent ID of this block. Root blocks don't have parent ids.
    pub parent_id: Option<String>,
    /// Mark the states of this block. It can be deleted, pinned, archived, etc.
    pub flags: Flags,
    /// Actual data contained in this block
    pub payloads: Vec<Payload>,
}

/// Next: do we store dynamic data? like hashmap?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// For displaying on the UI. It is the title of the block.
    pub title: Option<String>,
    /// The position (row, column) of the payload on a block.
    pub order: (usize, usize),
    /// Can be Text, Image bytes. extensible for other modality
    pub content: Content,
    pub created_at: usize,
    pub last_modified: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub value: Vec<u8>,
    pub vector: Vec<f32>,
    pub content_type: ContentType, // Content type presented in which style. For example, text can be P1 or so.
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Flags {
    pub is_deleted: bool, // Reserved for soft deletion
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContentType {
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
